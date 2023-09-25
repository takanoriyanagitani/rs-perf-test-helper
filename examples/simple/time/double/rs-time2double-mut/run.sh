#!/bin/sh

listen_addr=127.0.0.1:9251

RUST_LOG=info \
ENV_LISTEN_ADDR="${listen_addr}" \
	./rs-time2double-mut
