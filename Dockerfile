FROM ubuntu:latest
WORKDIR /app
COPY ./target/release/prover-server .
COPY ./libzktrie.so .
ENV LD_LIBRARY_PATH=/app/libzktrie.so:
EXPOSE 3030
CMD ["/bin/sh","-c","./prover-server"]
