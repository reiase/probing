cd app
trunk build --filehash false --release -M -d dist/
cd ..
cargo b -r
cargo b -r -p cli

test -e dist || mkdir dist
cp target/release/libprobe.so ./dist
cp target/release/probe       ./dist
