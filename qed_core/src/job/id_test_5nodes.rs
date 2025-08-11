#[cfg(test)]
mod test_5_nodes {
    use super::*;
    use plonky2::field::goldilocks_field::GoldilocksField;
    use plonky2::hash::hash_types::HashOut;
    use plonky2::plonk::config::PoseidonGoldilocksConfig;
    use plonky2::plonk::proof::ProofWithPublicInputs;
    use std::collections::HashMap;
    
    type F = GoldilocksField;
    type C = PoseidonGoldilocksConfig;
    const D: usize = 2;
    
    struct MockProofStore {
        proofs: HashMap<QProvingJobDataID, ProofWithPublicInputs<F, C, D>>,
    }
    
    impl MockProofStore {
        fn new() -> Self {
            Self {
                proofs: HashMap::new(),
            }
        }
        
        fn add_proof(&mut self, job_id: QProvingJobDataID, commitment: QHashOut<F>, parent_key: Option<QHashOut<F>>) {
            let mut public_inputs = vec![F::ZERO; 8];
            public_inputs[0] = commitment.0.elements[0];
            public_inputs[1] = commitment.0.elements[1];
            public_inputs[2] = commitment.0.elements[2];
            public_inputs[3] = commitment.0.elements[3];
            
            if let Some(key) = parent_key {
                public_inputs[4] = key.0.elements[0];
                public_inputs[5] = key.0.elements[1];
                public_inputs[6] = key.0.elements[2];
                public_inputs[7] = key.0.elements[3];
            }
            
            let proof = ProofWithPublicInputs {
                proof: Default::default(),
                public_inputs,
                public_inputs_target: vec![],
            };
            
            self.proofs.insert(job_id.get_output_id(), proof);
        }
    }
    
    impl crate::job::traits::QProofStore for MockProofStore {
        fn get_proof_by_id<C2: plonky2::plonk::config::GenericConfig<D2>, const D2: usize>(
            &self,
            id: QProvingJobDataID,
        ) -> anyhow::Result<ProofWithPublicInputs<C2::F, C2, D2>> {
            self.proofs.get(&id)
                .ok_or_else(|| anyhow::anyhow!("Proof not found"))
                .map(|p| unsafe { std::mem::transmute_copy(p) })
        }
        
        fn get_bytes_by_id(&self, _id: QProvingJobDataID) -> anyhow::Result<Vec<u8>> {
            Ok(vec![])
        }
        
        fn set_proof_by_id<C2: plonky2::plonk::config::GenericConfig<D2>, const D2: usize>(
            &mut self,
            _id: QProvingJobDataID,
            _proof: &ProofWithPublicInputs<C2::F, C2, D2>,
        ) -> anyhow::Result<()> {
            Ok(())
        }
        
        fn set_bytes_by_id(&mut self, _id: QProvingJobDataID, _data: &[u8]) -> anyhow::Result<()> {
            Ok(())
        }
        
        fn inc_counter_by_id(&mut self, _id: QProvingJobDataID) -> anyhow::Result<u32> {
            Ok(0)
        }
        
        fn write_next_jobs(
            &mut self,
            _jobs: &[QProvingJobDataID],
            _next_jobs: &[QProvingJobDataID],
        ) -> anyhow::Result<()> {
            Ok(())
        }
        
        fn write_multidimensional_jobs(
            &mut self,
            _jobs_levels: &[Vec<QProvingJobDataID>],
            _next_jobs: &[QProvingJobDataID],
        ) -> anyhow::Result<()> {
            Ok(())
        }
    }
    
    #[test]
    fn test_5_nodes_generate_and_verify_proof() {
        let mut graph = JobsTaskGraph::new();
        let mut proof_store = MockProofStore::new();
        
        let leaf_jobs: Vec<QProvingJobDataID> = (0..5)
            .map(|i| QProvingJobDataID::new_proof_job_id(
                1, 
                ProvingJobCircuitType::AddL1Deposit, 
                0, 
                i, 
                0
            ))
            .collect();
        
        let leaf_commitments: Vec<QHashOut<F>> = vec![
            QHashOut::from_values(1, 2, 3, 4),
            QHashOut::from_values(5, 6, 7, 8),
            QHashOut::from_values(9, 10, 11, 12),
            QHashOut::from_values(13, 14, 15, 16),
            QHashOut::from_values(17, 18, 19, 20),
        ];
        
        for (job, commitment) in leaf_jobs.iter().zip(leaf_commitments.iter()) {
            proof_store.add_proof(*job, *commitment, None);
        }
        
        let task0 = JobsTask::new(&leaf_jobs);
        graph.add_task(task0.clone());
        
        let parent_jobs = vec![
            QProvingJobDataID::new_proof_job_id(1, ProvingJobCircuitType::AddL1Deposit, 1, 0, 0),
            QProvingJobDataID::new_proof_job_id(1, ProvingJobCircuitType::AddL1Deposit, 1, 1, 0),
        ];
        
        let parent0_1_commitment = QHashOut(PoseidonHash::two_to_one(
            leaf_commitments[0].0,
            leaf_commitments[1].0,
        ));
        let parent2_3_commitment = QHashOut(PoseidonHash::two_to_one(
            leaf_commitments[2].0,
            leaf_commitments[3].0,
        ));
        
        proof_store.add_proof(parent_jobs[0], parent0_1_commitment, Some(QHashOut::from_values(100, 101, 102, 103)));
        proof_store.add_proof(parent_jobs[1], parent2_3_commitment, Some(QHashOut::from_values(104, 105, 106, 107)));
        
        let task1 = JobsTask::new(&parent_jobs);
        graph.add_dep(task1.clone(), task0.clone());
        
        let root_job = QProvingJobDataID::new_proof_job_id(1, ProvingJobCircuitType::AddL1Deposit, 2, 0, 0);
        let root_commitment = QHashOut(PoseidonHash::two_to_one(
            parent0_1_commitment.0,
            parent2_3_commitment.0,
        ));
        proof_store.add_proof(root_job, root_commitment, Some(QHashOut::from_values(200, 201, 202, 203)));
        
        let task2 = JobsTask::new(&[root_job]);
        graph.add_dep(task2.clone(), task1.clone());
        
        let final_root_job = QProvingJobDataID::new_proof_job_id(1, ProvingJobCircuitType::AddL1Deposit, 3, 0, 0);
        let final_root_commitment = QHashOut(PoseidonHash::two_to_one(
            root_commitment.0,
            leaf_commitments[4].0,
        ));
        proof_store.add_proof(final_root_job, final_root_commitment, Some(QHashOut::from_values(300, 301, 302, 303)));
        
        let task3 = JobsTask::new(&[final_root_job]);
        graph.add_dep(task3.clone(), task2.clone());
        graph.add_dep(task3.clone(), task0.clone());
        
        println!("\n=== Testing Job0 (has sibling) ===");
        let proof_job0 = graph.generate_proof(leaf_jobs[0], &proof_store).unwrap();
        println!("Job0 proof: {:?}", proof_job0);
        println!("Siblings count: {}", proof_job0.siblings.len());
        for (i, sibling) in proof_job0.siblings.iter().enumerate() {
            println!("  Sibling {}: is_left={}, has_parent_key={}", 
                i, sibling.is_left, sibling.parent_public_key.is_some());
        }
        assert!(graph.verify_proof(&proof_job0), "Job0 proof verification failed");
        
        println!("\n=== Testing Job4 (no sibling, promoted) ===");
        let proof_job4 = graph.generate_proof(leaf_jobs[4], &proof_store).unwrap();
        println!("Job4 proof: {:?}", proof_job4);
        println!("Siblings count: {}", proof_job4.siblings.len());
        for (i, sibling) in proof_job4.siblings.iter().enumerate() {
            println!("  Sibling {}: is_left={}, has_parent_key={}", 
                i, sibling.is_left, sibling.parent_public_key.is_some());
        }
        assert!(graph.verify_proof(&proof_job4), "Job4 proof verification failed");
        
        println!("\n=== Testing Job2 (middle node) ===");
        let proof_job2 = graph.generate_proof(leaf_jobs[2], &proof_store).unwrap();
        println!("Job2 proof: {:?}", proof_job2);
        assert!(graph.verify_proof(&proof_job2), "Job2 proof verification failed");
        
        println!("\nAll proofs verified successfully!");
    }
}