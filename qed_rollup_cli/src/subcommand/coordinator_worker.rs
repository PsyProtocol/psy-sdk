use qed_coordinator_node::CoordinatorWorkerArgs;

pub async fn run(args: CoordinatorWorkerArgs) -> anyhow::Result<()> {
    qed_coordinator_node::run_worker(args).await
}
