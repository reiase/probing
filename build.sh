cd app
trunk build --filehash false --release -M -d ../dist/
cd ..
cargo b -r
cargo b -r -p cli

test -e pkg || mkdir pkg
cp target/release/libprobe.so ./pkg
cp target/release/probe       ./pkg
