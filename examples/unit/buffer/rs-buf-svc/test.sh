#!/bin/sh

listen_addr=127.0.0.1:10151
protodir=rs-perf-helper-proto

reply_hi=20231001
reply_lo=73205

req2buf(){
	jq \
		-n \
		-c \
		--arg hi "${reply_hi}" \
		--arg lo "${reply_lo}" \
		'{
			request_id: {
				hi: 20231001,
				lo: 73123,
			},
			reply_id: {
				hi: $hi,
				lo: $lo,
			},
			req: {
				request_id: {
					hi: 20231001,
					lo: 73357,
				},
				seed: "aGVs",
			},
			received: "2023-09-30T22:35:03.0Z",
		}' |
		grpcurl \
		-plaintext \
		-d @ \
		-import-path "${protodir}" \
		-proto perf/helper/proto/buffer/v1/helper.proto \
		"${listen_addr}" \
		perf.helper.proto.buffer.v1.ReqBufferService/Save
}

buf2req(){
	jq \
		-n \
		-c \
		--arg hi "${reply_hi}" \
		--arg lo "${reply_lo}" \
		'{
			request_id: {
				hi: 20231001,
				lo: 73609,
			},
			retry: {
				retry_max: 4,
			},
		}' |
		grpcurl \
		-plaintext \
		-d @ \
		-import-path "${protodir}" \
		-proto perf/helper/proto/buffer/v1/helper.proto \
		"${listen_addr}" \
		perf.helper.proto.buffer.v1.ReqBufferService/Load
}

res2buf(){
	jq \
		-n \
		-c \
		--arg hi "${reply_hi}" \
		--arg lo "${reply_lo}" \
		'{
			request_id: {
				hi: 20231001,
				lo: 73123,
			},
			reply_id: {
				hi: $hi,
				lo: $lo,
			},
			res: {
				converted: "2023-09-30T22:35:05.0Z",
				generated: "SEVM",
			},
			received:  "2023-09-30T22:35:03.0Z",
			saved:     "2023-09-30T22:35:04.0Z",
			converted: "2023-09-30T22:35:05.0Z",
		}' |
		grpcurl \
		-plaintext \
		-d @ \
		-import-path "${protodir}" \
		-proto perf/helper/proto/buffer/v1/helper.proto \
		"${listen_addr}" \
		perf.helper.proto.buffer.v1.ResBufferService/Set
}

#req2buf
#buf2req
res2buf
