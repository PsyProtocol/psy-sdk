FROM rust:1.88.0-slim AS builder

RUN cargo install sqlx-cli \
    --no-default-features \
    --features rustls,postgres,mysql

FROM docker.io/library/ubuntu:24.04

ARG PROFILE=release

RUN apt update -y \
  && apt install -y ca-certificates libssl-dev tzdata jq wget curl docker.io vim net-tools postgresql-client \
  && apt clean \
  && rm -rf /var/lib/apt/lists/*

COPY --from=builder /usr/local/cargo/bin/sqlx /usr/local/bin/

WORKDIR /qed-rollup

COPY ./target/${PROFILE}/psy_node_cli /qed-rollup
COPY ./target/${PROFILE}/psy_user_cli /qed-rollup
COPY ./target/${PROFILE}/psy_dev_cli /qed-rollup
COPY ./target/${PROFILE}/psy_api_services /qed-rollup
COPY ./psy_api_services/migrations /qed-rollup/migrations
COPY .env /qed-rollup/.env

# Copy precompiles
COPY ./psy_precompiles/token           /qed-rollup/psy_precompiles/token
COPY ./psy_precompiles/rewards         /qed-rollup/psy_precompiles/rewards
COPY ./psy_precompiles/mining_rewards  /qed-rollup/psy_precompiles/mining_rewards


RUN echo '#!/bin/bash\n/qed-rollup/psy_node_cli $@' > /qed-rollup/.entrypoint.sh
RUN chmod u+x /qed-rollup/.entrypoint.sh

ENTRYPOINT ["/qed-rollup/.entrypoint.sh"]

