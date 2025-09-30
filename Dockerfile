FROM docker.io/library/ubuntu:24.04

ARG PROFILE=release

RUN apt update -y \
  && apt install -y ca-certificates libssl-dev tzdata jq wget curl docker.io vim net-tools

WORKDIR /qed-rollup

COPY ./target/${PROFILE}/qed_rollup_cli /qed-rollup
COPY ./target/${PROFILE}/qed_user_cli /qed-rollup
COPY ./target/${PROFILE}/qed_dev_cli /qed-rollup
COPY ./target/${PROFILE}/qed_api_services /qed-rollup

COPY ./qed_precompiles/token           /qed-rollup/precompiles/token
COPY ./qed_precompiles/rewards         /qed-rollup/precompiles/rewards
COPY ./qed_precompiles/mining_rewards  /qed-rollup/precompiles/mining_rewards


RUN echo '#!/bin/bash\n/qed-rollup/qed_rollup_cli $@' > /qed-rollup/.entrypoint.sh
RUN chmod u+x /qed-rollup/.entrypoint.sh

ENTRYPOINT ["/qed-rollup/.entrypoint.sh"]

