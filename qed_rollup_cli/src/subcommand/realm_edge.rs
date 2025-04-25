use qed_realm_node::RealmEdgeConfig;

pub async fn run(args: RealmEdgeConfig) -> anyhow::Result<()> {
    qed_realm_node::run_realm_edge(args).await
}
