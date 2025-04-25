use qed_coordinator_node::CoordinatorEdgeArgs;

pub async fn run(args: CoordinatorEdgeArgs) -> anyhow::Result<()> {
    qed_coordinator_node::run_edge(args).await
}
