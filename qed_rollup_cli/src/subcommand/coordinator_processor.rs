use qed_coordinator_node::CoordinatorProcessorArgs;

pub async fn run(args: CoordinatorProcessorArgs) -> anyhow::Result<()> {
    qed_coordinator_node::run_processor(args).await
}
