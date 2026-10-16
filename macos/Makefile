BUILD_DIR := .build
MODEL_DST := Govorun/Resources/Model

SHERPA_VER := v1.13.1
SHERPA_URL := https://github.com/k2-fsa/sherpa-onnx/releases/download/$(SHERPA_VER)/sherpa-onnx-$(SHERPA_VER)-macos-xcframework-static.tar.bz2
SHERPA_SHA256 := c4323fda2c17da3e5f820e98dd1208743ebd8e8fde35cc5e2ac0b9cff98cab35
SHERPA_ARCHIVE_ROOT := sherpa-onnx-$(SHERPA_VER)-macos-xcframework-static
ORT_VER := 1.24.4
ORT_URL := https://github.com/microsoft/onnxruntime/releases/download/v$(ORT_VER)/onnxruntime-osx-arm64-$(ORT_VER).tgz
ORT_SHA256 := 93787795f47e1eee369182e43ed51b9e5da0878ab0346aecf4258979b8bba989
ORT_ARCHIVE_ROOT := onnxruntime-osx-arm64-$(ORT_VER)
TRANSCRIBE_VER := 0.2.1
TRANSCRIBE_URL := https://github.com/handy-computer/transcribe.cpp/releases/download/v$(TRANSCRIBE_VER)/TranscribeCpp.xcframework.zip
TRANSCRIBE_SHA256 := d24e6c0aaff1e628a626f792f74bb7155287a49a5c5bb1179deb73b35f0410f5
GIGAAM_FILE := gigaam-v3-e2e-rnnt-Q8_0.gguf
GIGAAM_URL := https://huggingface.co/handy-computer/gigaam-v3-e2e-rnnt-gguf/resolve/main/$(GIGAAM_FILE)
GIGAAM_SHA256 := 78d63b47723b7f8d78c6113a6ef983b5a86e2a86f6c273e1f5cb6967b1c4467a
SILERO_URL := https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/silero_vad.onnx
SILERO_SHA256 := 9e2449e1087496d8d4caba907f23e0bd3f78d91fa552479bb9c23ac09cbb1fd6

.PHONY: setup setup-sherpa setup-transcribe setup-model setup-benchmark build clean

setup: setup-sherpa setup-transcribe setup-model setup-benchmark

setup-sherpa:
	@bash ./scripts/fetch-verified.sh "$(SHERPA_URL)" "$(SHERPA_SHA256)" ".cache/sherpa-onnx.tar.bz2"
	@bash ./scripts/fetch-verified.sh "$(ORT_URL)" "$(ORT_SHA256)" ".cache/onnxruntime.tgz"
	@rm -rf -- ".cache/sherpa-unpack" ".cache/onnxruntime-unpack"
	@mkdir -p Frameworks .cache/sherpa-unpack .cache/onnxruntime-unpack
	@tar -xjf .cache/sherpa-onnx.tar.bz2 -C .cache/sherpa-unpack
	@tar -xzf .cache/onnxruntime.tgz -C .cache/onnxruntime-unpack
	@rm -rf -- Frameworks/sherpa-onnx.xcframework
	@cp -R ".cache/sherpa-unpack/$(SHERPA_ARCHIVE_ROOT)/sherpa-onnx.xcframework" Frameworks/
	@cp BuildSupport/SherpaOnnx.modulemap Frameworks/sherpa-onnx.xcframework/macos-arm64_x86_64/Headers/module.modulemap
	@rm -f -- Frameworks/libonnxruntime.*.dylib
	@cp ".cache/onnxruntime-unpack/$(ORT_ARCHIVE_ROOT)/lib/libonnxruntime.$(ORT_VER).dylib" Frameworks/

setup-transcribe:
	@bash ./scripts/fetch-verified.sh "$(TRANSCRIBE_URL)" "$(TRANSCRIBE_SHA256)" ".cache/transcribe.zip"
	@rm -rf -- ".cache/transcribe-unpack"
	@mkdir -p Frameworks .cache/transcribe-unpack
	@unzip -q -o .cache/transcribe.zip -d .cache/transcribe-unpack
	@rm -rf -- Frameworks/TranscribeCpp.xcframework
	@cp -R .cache/transcribe-unpack/TranscribeCpp.xcframework Frameworks/

setup-model:
	@mkdir -p "$(MODEL_DST)"
	@bash ./scripts/fetch-verified.sh "$(GIGAAM_URL)" "$(GIGAAM_SHA256)" "$(MODEL_DST)/$(GIGAAM_FILE)"
	@bash ./scripts/fetch-verified.sh "$(SILERO_URL)" "$(SILERO_SHA256)" "$(MODEL_DST)/silero_vad.onnx"

setup-benchmark:
	@mkdir -p "Govorun/Resources/Benchmark"
	@bash ./scripts/fetch-verified.sh "https://raw.githubusercontent.com/amidexe/govorun-lite/91444be6316a17a05054cd3f815ae15a02e2dda8/app/src/main/assets/benchmark.wav" "d8aaaa18a5098d7c6de0595ae7ac1e64cacd0d4022af3595213bdaf23be77e69" "Govorun/Resources/Benchmark/benchmark.wav"
	@bash ./scripts/fetch-verified.sh "https://raw.githubusercontent.com/amidexe/govorun-lite/91444be6316a17a05054cd3f815ae15a02e2dda8/app/src/main/assets/benchmark_long.wav" "1868ece0195dfa9fc2394be24865d1133c8452a8292a397db45ba8c3ed9e01e3" "Govorun/Resources/Benchmark/benchmark_long.wav"

build: setup
	GOVORUN_SHERPA_RUNTIME="$(CURDIR)/Frameworks" python3 ../desktop/scripts/build-macos.py

clean:
	rm -rf -- "$(BUILD_DIR)" .cache Frameworks "$(MODEL_DST)" "Govorun.xcodeproj"
