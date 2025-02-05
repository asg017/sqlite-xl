SHELL := /bin/bash

VERSION=$(shell cat VERSION)

ifeq ($(shell uname -s),Darwin)
CONFIG_DARWIN=y
else ifeq ($(OS),Windows_NT)
CONFIG_WINDOWS=y
else
CONFIG_LINUX=y
endif

LIBRARY_PREFIX=lib
ifdef CONFIG_DARWIN
LOADABLE_EXTENSION=dylib
STATIC_EXTENSION=a
endif

ifdef CONFIG_LINUX
LOADABLE_EXTENSION=so
STATIC_EXTENSION=a
endif


ifdef CONFIG_WINDOWS
LOADABLE_EXTENSION=dll
LIBRARY_PREFIX=
STATIC_EXTENSION=lib
endif

ifdef target
CARGO_TARGET=--target=$(target)
BUILT_LOCATION=target/$(target)/debug/$(LIBRARY_PREFIX)sqlite_xl.$(LOADABLE_EXTENSION)
BUILT_LOCATION_RELEASE=target/$(target)/release/$(LIBRARY_PREFIX)sqlite_xl.$(LOADABLE_EXTENSION)
BUILT_LOCATION_STATIC=target/$(target)/debug/$(LIBRARY_PREFIX)sqlite_xl.$(STATIC_EXTENSION)
BUILT_LOCATION_STATIC_RELEASE=target/$(target)/release/$(LIBRARY_PREFIX)sqlite_xl.$(STATIC_EXTENSION)
else
CARGO_TARGET=
BUILT_LOCATION=target/debug/$(LIBRARY_PREFIX)sqlite_xl.$(LOADABLE_EXTENSION)
BUILT_LOCATION_RELEASE=target/release/$(LIBRARY_PREFIX)sqlite_xl.$(LOADABLE_EXTENSION)
BUILT_LOCATION_STATIC=target/debug/$(LIBRARY_PREFIX)sqlite_xl.$(STATIC_EXTENSION)
BUILT_LOCATION_STATIC_RELEASE=target/release/$(LIBRARY_PREFIX)sqlite_xl.$(STATIC_EXTENSION)
endif


ifdef python
PYTHON=$(python)
else
PYTHON=python3
endif

prefix=dist
TARGET_LOADABLE=$(prefix)/debug/xl0.$(LOADABLE_EXTENSION)
TARGET_LOADABLE_RELEASE=$(prefix)/release/xl0.$(LOADABLE_EXTENSION)

TARGET_STATIC=$(prefix)/debug/xl.a
TARGET_STATIC_RELEASE=$(prefix)/release/xl0.a


ifdef target
CARGO_TARGET=--target=$(target)
BUILT_LOCATION=target/$(target)/debug/$(LIBRARY_PREFIX)sqlite_xl.$(LOADABLE_EXTENSION)
BUILT_LOCATION_RELEASE=target/$(target)/release/$(LIBRARY_PREFIX)sqlite_xl.$(LOADABLE_EXTENSION)
else
CARGO_TARGET=
BUILT_LOCATION=target/debug/$(LIBRARY_PREFIX)sqlite_xl.$(LOADABLE_EXTENSION)
BUILT_LOCATION_RELEASE=target/release/$(LIBRARY_PREFIX)sqlite_xl.$(LOADABLE_EXTENSION)
endif


$(prefix):
	mkdir -p $(prefix)/debug
	mkdir -p $(prefix)/release


$(TARGET_LOADABLE): $(prefix) $(shell find . -type f -name '*.rs')
	cargo build $(CARGO_TARGET)
	cp $(BUILT_LOCATION) $@

$(TARGET_LOADABLE_RELEASE): $(prefix) $(shell find . -type f -name '*.rs')
	cargo build --release $(CARGO_TARGET)
	cp $(BUILT_LOCATION_RELEASE) $@

Cargo.toml: VERSION
	cargo set-version `cat VERSION`


version:
	make Cargo.toml

format:
	cargo fmt


release: $(TARGET_LOADABLE_RELEASE) $(TARGET_STATIC_RELEASE)

loadable: $(TARGET_LOADABLE)
loadable-release: $(TARGET_LOADABLE_RELEASE)

static: $(TARGET_STATIC) $(TARGET_H)
static-release: $(TARGET_STATIC_RELEASE) $(TARGET_H_RELEASE)

debug: loadable static python datasette
release: loadable-release static-release python-release datasette-release

clean:
	rm dist/*
	cargo clean

publish-release:
	./scripts/publish_release.sh

.PHONY: clean \
	test test-loadable \
	loadable loadable-release \
	static static-release \
	debug release \
	format version publish-release

# ███████████████████████████████ WASM SECTION ███████████████████████████████

SQLITE_WASM_VERSION=3440000
SQLITE_WASM_YEAR=2023
SQLITE_WASM_SRCZIP=$(prefix)/sqlite-src.zip
SQLITE_WASM_COMPILED_SQLITE3C=$(prefix)/sqlite-src-$(SQLITE_WASM_VERSION)/sqlite3.c
SQLITE_WASM_COMPILED_MJS=$(prefix)/sqlite-src-$(SQLITE_WASM_VERSION)/ext/wasm/jswasm/sqlite3.mjs
SQLITE_WASM_COMPILED_WASM=$(prefix)/sqlite-src-$(SQLITE_WASM_VERSION)/ext/wasm/jswasm/sqlite3.wasm

TARGET_WASM_LIB=$(prefix)/libsqlite_xl.wasm.a
TARGET_WASM_MJS=$(prefix)/sqlite3.mjs
TARGET_WASM_WASM=$(prefix)/sqlite3.wasm
TARGET_WASM=$(TARGET_WASM_MJS) $(TARGET_WASM_WASM)

$(SQLITE_WASM_SRCZIP):
	curl -o $@ https://www.sqlite.org/$(SQLITE_WASM_YEAR)/sqlite-src-$(SQLITE_WASM_VERSION).zip

$(SQLITE_WASM_COMPILED_SQLITE3C): $(SQLITE_WASM_SRCZIP)
	unzip -q -o $< -d $(prefix)
	(cd $(prefix)/sqlite-src-$(SQLITE_WASM_VERSION)/ && ./configure --enable-all && make sqlite3.c)

$(TARGET_WASM_LIB): $(shell find src -type f -name '*.rs')
	RUSTFLAGS="-Clink-args=-sERROR_ON_UNDEFINED_SYMBOLS=0 -Clink-args=--no-entry" \
		cargo build --release --target wasm32-unknown-emscripten --features=static
		cp target/wasm32-unknown-emscripten/release/libsqlite_xl.a $@

$(SQLITE_WASM_COMPILED_MJS) $(SQLITE_WASM_COMPILED_WASM): $(SQLITE_WASM_COMPILED_SQLITE3C) $(TARGET_WASM_LIB)
	(cd $(prefix)/sqlite-src-$(SQLITE_WASM_VERSION)/ext/wasm && \
		make sqlite3_wasm_extra_init.c=../../../libsqlite_xl.wasm.a "emcc.flags=-s EXTRA_EXPORTED_RUNTIME_METHODS=['ENV'] -s FETCH")

$(TARGET_WASM_MJS): $(SQLITE_WASM_COMPILED_MJS)
	cp $< $@

$(TARGET_WASM_WASM): $(SQLITE_WASM_COMPILED_WASM)
	cp $< $@

wasm: $(TARGET_WASM)

# ███████████████████████████████   END WASM   ███████████████████████████████


# ███████████████████████████████ SITE SECTION ███████████████████████████████

WASM_TOOLKIT_NPM_TGZ=$(prefix)/sqlite-wasm-toolkit-npm.tgz

TARGET_SITE_DIR=$(prefix)/site
TARGET_SITE=$(prefix)/site/index.html

$(WASM_TOOLKIT_NPM_TGZ):
	curl -o $@ -q https://registry.npmjs.org/@alex.garcia/sqlite-wasm-toolkit/-/sqlite-wasm-toolkit-0.0.1-alpha.8.tgz

$(TARGET_SITE_DIR)/slim.js $(TARGET_SITE_DIR)/slim.css: $(WASM_TOOLKIT_NPM_TGZ)
	tar -xvzf $< -C $(TARGET_SITE_DIR) --strip-components=2 package/dist/slim.js package/dist/slim.css


$(TARGET_SITE_DIR):
	mkdir -p $(TARGET_SITE_DIR)

$(TARGET_SITE): site/index.html $(TARGET_SITE_DIR) $(TARGET_WASM_MJS) $(TARGET_WASM_WASM) $(TARGET_SITE_DIR)/slim.js $(TARGET_SITE_DIR)/slim.css
	cp $(TARGET_WASM_MJS) $(TARGET_SITE_DIR)
	cp $(TARGET_WASM_WASM) $(TARGET_SITE_DIR)
	cp tests/file-sample.xlsx $(TARGET_SITE_DIR)
	cp $< $@
site: $(TARGET_SITE)
# ███████████████████████████████   END SITE   ███████████████████████████████
