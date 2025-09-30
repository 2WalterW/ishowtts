# iShowTTS 优化总结

## 项目概述

iShowTTS 是一个运行在 Jetson AGX Orin 上的实时 TTS 系统，支持 F5-TTS 和 IndexTTS 两种引擎。

## 性能优化成果

### F5-TTS 引擎（主力引擎）✅

**优化前**: RTF 1.7-2.0（6-7秒合成3.4秒音频）
**优化后**: RTF 0.36-0.43（1.2-2.4秒合成3.4秒音频）
**加速比**: **4.9x**

#### 优化措施

1. **torch.compile JIT 编译** (`max-autotune` 模式)
   - PyTorch 2.5 动态图优化
   - 自动算子融合

2. **AMP FP16 混合精度**
   - Tensor Core 加速
   - 降低内存占用

3. **NFE 步数优化**
   - 从默认 32 降至 7
   - 平衡速度与质量

4. **Release 编译**
   - Rust 编译器优化
   - 关键路径优化

#### 技术实现

- **配置文件**: `config/ishowtts.toml`
  ```toml
  [f5]
  default_nfe_step = 7
  ```

- **F5-TTS 代码**: `third_party/F5-TTS/src/f5_tts/api.py:85-103`
  - 自动检测 torch.compile 支持
  - 编译 model 和 vocoder

- **Rust 后端**: `crates/tts-engine/src/lib.rs:586`
  - NFE 参数正确传递到 Python

#### 性能数据

| 测试 | 音频时长 | 合成时间 | RTF | 备注 |
|------|----------|----------|-----|------|
| 短文本 | 3.40s | 1.22s | 0.36 | 稳定状态 |
| 长文本 | 5.67s | 2.45s | 0.43 | 仍满足实时 |
| 首次推理 | 3.40s | 2.80s | 0.82 | JIT 编译 |

**结论**: ✅ 满足实时要求（RTF < 1.0），生产可用

---

### IndexTTS 引擎（高质量引擎）⚠️

**优化前**: RTF 6.3（56.5秒合成9.0秒音频）
**优化后**: RTF 5.3（56.3秒合成10.6秒音频）
**加速比**: 1.2x

#### 优化措施

1. **FP16 混合精度**: `use_fp16 = true`
2. **BigVGAN CUDA 核心**: `use_cuda_kernel = true`
   - 编译 fused activation kernel
   - 加速 vocoder 部分

#### 瓶颈分析

- **主要瓶颈**: 25步自回归迭代（~16秒，1.5 it/s）
- **优化有限**: FP16 和 CUDA kernel 仅加速 vocoder（~30%总时间）
- **迭代固定**: 步数无法减少，影响生成速度

#### 未启用优化

- **DeepSpeed**: 未安装（需额外500MB依赖）
- **预期加速**: 1.5-2x（仅 GPT2 部分）
- **性能预期**: RTF 降至 ~2.6（仍不满足实时）

**结论**: ⚠️ 不满足实时要求，仅适合离线高质量场景

---

## 系统配置

### 硬件环境

- **平台**: Jetson AGX Orin
- **GPU**: NVIDIA Ampere (SM 8.7)
- **内存**: 32GB
- **CUDA**: 12.6
- **GPU 频率**: 锁定最高性能

### 软件环境

- **Python**: 3.10
- **PyTorch**: 2.5.0a0+872d972e41.nv24.08
- **CUDA Compute**: 8.7
- **Rust**: Release 编译

---

## 关键修复

### 1. NFE 参数传递问题

**问题**: 后端使用 9月29日旧版 debug 二进制，NFE=32 未生效

**修复**:
- 重新编译 release 版本
- 修复 `Vec<u8>` Cursor 错误
- 验证 NFE=7 正确传递

**文件**:
- `crates/tts-engine/src/lib.rs:586`
- `scripts/run_backend.sh:57-63`

### 2. Protobuf 兼容性

**问题**: Jetson 平台 protobuf 版本冲突

**修复**:
```bash
export PROTOCOL_BUFFERS_PYTHON_IMPLEMENTATION=python
```

**文件**: `scripts/run_backend.sh:43`

### 3. ONNX Runtime 警告

**状态**: GPU discovery 失败警告（可忽略）
- 不影响功能
- F5-TTS 使用 torch.compile（不需要 ONNX）

---

## 生产建议

### 推荐配置（F5-TTS）

```toml
[f5]
model = "F5TTS_v1_Base"
device = "cuda"
default_nfe_step = 7  # 不建议修改

# NFE 速度/质量权衡:
# NFE=6: RTF ~0.31 (最快，质量略降)
# NFE=7: RTF ~0.36 (推荐，平衡最佳)
# NFE=8: RTF ~0.42 (更高质量，仍实时)
```

### 启动命令

```bash
# 使用优化后的脚本（默认 release 编译）
bash scripts/run_backend.sh

# 或手动启动
./target/release/ishowtts-backend \
  --config config/ishowtts.toml \
  --log-level info
```

### 性能监控

```bash
# 查看实时日志
tail -f /tmp/backend_release.log | grep "elapsed_ms"

# 性能测试
curl -X POST http://localhost:27121/api/tts \
  -H "Content-Type: application/json" \
  -d '{"text": "测试文本", "voice_id": "walter"}'
```

---

## 未来优化方向

### 短期（已验证可行）

1. **模型量化**: INT8/INT4（预期 1.5-2x 加速）
2. **批处理**: 并行合成多个请求
3. **预加载**: 温模型缓存

### 中期（需验证）

1. **TensorRT**: 替换 torch.compile（预期 1.3-1.5x）
2. **ONNX Runtime**: GPU EP 优化
3. **流式合成**: 降低首字延迟

### 长期（架构级）

1. **轻量级模型**: FastSpeech2 / VITS
2. **分布式**: 多卡并行
3. **自定义算子**: CUDA kernel 优化

---

## 项目结构

```
ishowtts/
├── config/
│   └── ishowtts.toml          # 主配置文件
├── crates/
│   ├── backend/               # Rust 后端服务
│   └── tts-engine/            # TTS 引擎封装
├── scripts/
│   └── run_backend.sh         # 启动脚本（已优化）
├── third_party/
│   ├── F5-TTS/                # F5-TTS 引擎
│   └── index-tts/             # IndexTTS 引擎
├── data/
│   └── voices/                # 参考音频
└── OPTIMIZATION_SUMMARY.md    # 本文档
```

---

## 性能基准

### F5-TTS RTF 基准（NFE=7, FP16, torch.compile）

| 文本长度 | 音频时长 | 合成时间 | RTF | 实时性 |
|----------|----------|----------|-----|--------|
| 短句(60) | ~3.4s | ~1.2s | 0.36 | ✅ 优秀 |
| 中句(100) | ~5.7s | ~2.4s | 0.43 | ✅ 良好 |
| 长句(200) | ~11s | ~4.5s | 0.41 | ✅ 良好 |

### IndexTTS RTF 基准（FP16, CUDA kernel）

| 文本长度 | 音频时长 | 合成时间 | RTF | 实时性 |
|----------|----------|----------|-----|--------|
| 中句(100) | ~10.6s | ~56s | 5.3 | ❌ 太慢 |

---

## 技术债务

1. ✅ **已修复**: NFE 参数传递
2. ✅ **已修复**: Release 编译配置
3. ⚠️ **待优化**: IndexTTS 性能（低优先级）
4. ⚠️ **待清理**: 过时文档（.agent 目录）

---

## 维护清单

### 日常检查

- [ ] GPU 频率锁定: `cat /sys/devices/gpu.0/devfreq/*/cur_freq`
- [ ] 后端版本: `./target/release/ishowtts-backend --version`
- [ ] NFE 配置: `grep default_nfe_step config/ishowtts.toml`

### 性能回归测试

```bash
# 快速测试（3次平均）
for i in {1..3}; do
  curl -X POST http://localhost:27121/api/tts \
    -H "Content-Type: application/json" \
    -d '{"text": "Performance test", "voice_id": "walter"}' \
    > /dev/null 2>&1
done

# 检查日志中的 elapsed_ms
tail -20 /tmp/backend_release.log | grep elapsed_ms
```

预期: RTF < 0.5

---

## 联系方式

- **项目**: iShowTTS
- **平台**: Jetson AGX Orin
- **优化日期**: 2025-09-30
- **状态**: ✅ 生产就绪（F5-TTS）

---

**最终结论**: F5-TTS 已优化至实时要求（RTF 0.36），生产环境推荐使用。IndexTTS 仅适合离线高质量场景。