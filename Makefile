ENV = development

ASSETS_DIR = assets
BUILD_DIR = .build
CONTENT_DIR = content
WASM_OUT_DIR = pkg
JS_OUT_DIR = lib
STYLES_DIR = styles

ASSETS_BUILD_DIR = $(BUILD_DIR)/$(ASSETS_DIR)
JS_BUILD_DIR = $(ASSETS_BUILD_DIR)/scripts
STYLES_OUT_DIR = $(BUILD_DIR)/$(STYLES_DIR)

all: build run

prepare:
	-cargo install wasm-pack
	-rustup component add rustfmt
	wasm-pack build
	cd $(WASM_OUT_DIR) && npm link && cd ..
	npm link rusty-sketch
	npm install

clean:
	rm -rf pkg $(BUILD_DIR) lib

build: clean
	mkdir -p $(ASSETS_BUILD_DIR)
	mkdir -p $(JS_BUILD_DIR)

	cp -rf $(CONTENT_DIR)/* $(BUILD_DIR)/

	cargo fmt
	wasm-pack build
	NODE_ENV=$(ENV) npm run build

	# cp -r ./$(JS_OUT_DIR)/* $(JS_BUILD_DIR)/
	mv $(STYLES_OUT_DIR) $(ASSETS_BUILD_DIR)/

run: build
	NODE_ENV=$(ENV) npm run start

.PHONY: all prepare clean build
