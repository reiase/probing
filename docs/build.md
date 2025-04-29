## 环境准备
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh # 安装rust环境
rustup toolchain install nightly # 安装nightly版本工具链
rustup default nightly           # 切换工具链至 nightly 版本
rustup target add wasm32-unknown-unknown # 安装wasm32 target

cargo install cargo-zigbuild # 安装zigbuild，用于支持跨glibc版本的构建
pip install ziglang          # 安装zig工具链，pip安装方式比较简单，但是未测试过
```

```bash
#构建发布包
make ZIG=1

#安装构建包，必须force install，否则可能导致更新不成功
pip install dist/probing-0.2.0-py3-none-manylinux_2_12_x86_64.manylinux2010_x86_64.whl --force-reinstall 
```

```bash
# 测试imagenet训练
PROBE=1 python examples/test_imagenet.py -a resnet18 --dummy -b 1

# 跟踪train函数中的loss与acc1两个变量
PROBE_TORCH_EXPRS="loss@train,acc1@train" PROBE=1 python examples/test_imagenet.py -a resnet18 --dummy -b 1
```

## 常用命令
```bash
probing list # 查看本机已经被注入探针的进程
probing <pid> query "show tables" #查看进程<pid>中有哪些数据（表）
probing <pid> query "select * from python.torch_trace" # 查看对torch模型的trace
probing <pid> query "select * from python.variables" # 查看被trace的变量
```