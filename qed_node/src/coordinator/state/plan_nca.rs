use std::collections::HashSet;

use plonky2::hash::hash_types::RichField;
use qed_core::{data::qhashout::QHashOut, job::traits::QProofStore};
use qed_crypto::hash::merkle::utils::sub_tree_nca::UpdateNCAProofsWithDependencies;
use qed_data::guta::api::SubmitGUTARealmResultAPIQueueItem;

/*
pub fn plan_proofs_for_nca<PS: QProofStore, F: RichField>(ps: &mut PS, nca_update: &UpdateNCAProofsWithDependencies<QHashOut<F>>, req_items: &[SubmitGUTARealmResultAPIQueueItem<F>]){
    let total = nca_update.nca_proofs.len();
    let levels = nca_update.get_index_levels();

    let mut level_ind = 0;

    for i in levels[0].iter(). {

    }
    todo!()


    let mut total_finished = 0;
    let mut finished = HashSet::<i64>::new();
    let mut remaining_inds = (0..total).collect::<Vec<_>>();
    

    let mut rung_0_jobs = Vec::with_capacity(req_items.len());

    for i in req_items{
        
    }

    while total_finished< total {
        let mut chopping_block = Vec::new();
        let mut next_remaining_inds = Vec::new();
        for i in remaining_inds {
            let (l_dep, r_dep) = nca_update.dependencies[i];
            if (l_dep == -1 || finished.contains(&l_dep)) && (r_dep == -1 || finished.contains(&r_dep)) {
                chopping_block.push(i);
                //finished.insert(i);
                total_finished += 1;

                nca_update.req[i]
            }else{
                next_remaining_inds.push(i)
            }
        }
        chopping_block.into_iter().for_each(|i|{
            finished.insert(i as i64);
        });
        remaining_inds = next_remaining_inds;
    }
}
*/
