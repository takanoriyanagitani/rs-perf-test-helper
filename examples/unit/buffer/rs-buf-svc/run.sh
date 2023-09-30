#!/bin/sh

listen_addr=127.0.0.1:10151

ENV_LISTEN_ADDR="${listen_addr}" \
	./rs-buf-svc
