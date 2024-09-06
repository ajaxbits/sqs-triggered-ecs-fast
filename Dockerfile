FROM amazonlinux:2023 AS rust
RUN yum groupinstall -y "Development Tools"
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
RUN curl -LsSf https://astral.sh/uv/install.sh | sh
RUN 
RUN /root/.cargo/bin/uv python install 3.9 3.10 3.11 3.12
COPY ./ ./
RUN source $HOME/.cargo/env && PYO3_CONFIG_FILE="`pwd`/pyo3configtest" cargo build --release

FROM amazonlinux:2023 AS base
COPY --from=rust /target/release/fastest-ecs-scheduler ./.
COPY --from=rust /root/.local/share/uv /root/.local/share/uv
COPY --from=rust /root/.cargo/bin/uv /bin/uv
COPY --from=rust lambda /to-execute

ENV AWS_DEFAULT_REGION=us-east-1
run chmod +x ./fastest-ecs-scheduler
RUN cd /to-execute && \
    LD_LIBRARY_PATH="$(dirname $(dirname $(uv python find 3.12)))/lib" \
    PYTHONPATH="$(pwd):$(dirname $(dirname $(uv python find 3.12)))/lib/python3.12" \
    NUM_PROCESSES=1000 \
    FUNCTION_ENTRYPOINT="backup_target_list_advisor_handler.code" \
    ../fastest-ecs-scheduler
