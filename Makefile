prefix=dist

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
	cp $< $@
site: $(TARGET_SITE)
# ███████████████████████████████   END SITE   ███████████████████████████████
