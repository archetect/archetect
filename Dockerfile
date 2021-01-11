FROM rust:1.49.0 as build

COPY . .
RUN cargo build --package archetect-cli --release

FROM debian:9.11

ARG user=archetect
ARG group=archetect
ARG uid=1000
ARG gid=1000

COPY --from=build /target/release/archetect /bin/archetect
RUN chmod +x /bin/archetect

RUN apt-get -y update && apt-get install -y openssl git && \
    apt-get autoremove -y && apt-get clean && rm -rf /var/lib/apt/lists/* /tmp/* /var/tmp/*

RUN groupadd -g ${gid} ${group}
ENV HOME /home/${user}
RUN useradd -d $HOME -u ${uid} -g ${gid} -m ${user}
USER ${user}
WORKDIR ${HOME}

ENTRYPOINT ["archetect"]
