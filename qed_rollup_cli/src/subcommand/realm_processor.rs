use qed_realm_node::RealmNodeConfig;

pub async fn run(args: RealmNodeConfig) -> anyhow::Result<()> {
    qed_realm_node::run_realm_processor(args).await
}
