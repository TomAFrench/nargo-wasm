#!/usr/bin/env bash

if [ -v out ]; then
  echo "Will install package to $out (defined outside installPhase.sh script)"
else
  out="./result"
  echo "Will install package to $out"
fi

COMMIT_SHORT=$(git rev-parse --short HEAD)
VERSION_APPENDIX=""
if [ -n "$COMMIT_SHORT" ]; then
    VERSION_APPENDIX="-$COMMIT_SHORT"
else
    VERSION_APPENDIX="-NOGIT"
fi

# Extract version from Cargo.toml using toml2json
PACKAGE_VERSION=$(toml2json < Cargo.toml | jq -r .package.version)
if [ -z "$PACKAGE_VERSION" ]; then
    echo "Could not extract version from Cargo.toml"
    exit 1
fi
PACKAGE_VERSION+=$VERSION_APPENDIX

mkdir -p $out
cp README.md $out/
cp -r ./pkg/* $out/
cat package.json \
| jq --arg ver "$PACKAGE_VERSION" '.version = $ver 
| .repository = {"type": "git","url": "https://github.com/noir-lang/acvm-simulator-wasm.git"} 
| .sideEffects = false | .files = ["nodejs","web","package.json"] 
| .main = "./nodejs/acvm_simulator.js" 
| .types = "./web/acvm_simulator.d.ts" 
| .module = "./web/acvm_simulator.js"' \
> $out/package.json