#!/bin/bash
export ANCHOR_PROVIDER_URL=http://localhost:8899
export ANCHOR_WALLET='/Users/danielpyrathon/.config/solana/id.json'
npx ts-mocha -p ./tsconfig.json -t 1000000 'tests/*.test.ts' --watch-extensions ts --watch --watch-files 'tests/*.ts'
