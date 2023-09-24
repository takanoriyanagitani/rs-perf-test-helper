#!/bin/sh

listen_addr=127.0.0.1:9241

ENV_LISTEN_ADDR="${listen_addr}" \
	./rs-time2json
