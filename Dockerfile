FROM docker.io/library/ubuntu:22.04

ARG PROFILE=release

RUN apt update -y \
  && apt install -y ca-certificates libssl-dev tzdata

WORKDIR /qed-rollup

COPY ./target/${PROFILE}/qed_rollup_cli /qed-rollup
COPY ./target/${PROFILE}/qed_user_cli /qed-rollup
COPY ./target/${PROFILE}/qed_dev_cli /qed-rollup

RUN echo '#!/bin/bash\n/qed-rollup/qed_rollup_cli $@' > /qed-rollup/.entrypoint.sh
RUN chmod u+x /qed-rollup/.entrypoint.sh

ENTRYPOINT ["/qed-rollup/.entrypoint.sh"]

