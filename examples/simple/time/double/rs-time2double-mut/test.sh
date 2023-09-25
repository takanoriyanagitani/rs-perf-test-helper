#!/bin/sh

listen_addr=127.0.0.1:9251
protodir="./rs-perf-helper-proto"

which grpcurl | fgrep --silent grpcurl || exec echo 'grpcurl missing'
which jq      | fgrep --silent jq      || exec echo 'jq missing'
which python3 | fgrep --silent python3 || exec echo 'python3 missing'
which base64  | fgrep --silent base64  || exec echo 'base64 missing'

unixtime_us_base64=$(
	python3 \
		-c 'import sys; import struct; import time; import functools; functools.reduce(
			lambda state, f: f(state),
			[
				lambda f: f*1e6,
				int,
				struct.Struct(">Q").pack,
				sys.stdout.buffer.write,
			],
			time.time(),
		)' \
		| base64
)

conv(){
	echo "${unixtime_us_base64}" | base64 --decode | xxd

	jq \
		-n \
		-c \
		--arg time_us_b64 "${unixtime_us_base64}" \
		'{
			request_id: {
				hi: 20230924,
				lo: 85011,
			},
			seed: $time_us_b64,
		}' |
		grpcurl \
			-plaintext \
			-d @ \
			-import-path "${protodir}" \
			-proto perf/helper/proto/direct/v1/helper.proto \
			"${listen_addr}" \
			perf.helper.proto.direct.v1.ConvertService/Convert \
		| jq \
			--raw-output \
			.generated \
		| base64 --decode \
		| xxd
}

conv
