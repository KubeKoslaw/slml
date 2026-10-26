# Build and install the `slml` compiler from source, like any C compiler:
#
#   make                      build
#   make install              install to ~/.local (bin + man page)
#   sudo make install PREFIX=/usr/local
#   make install VIEWER=0     without the native window (servers, CI)

PREFIX ?= $(HOME)/.local
BINDIR ?= $(PREFIX)/bin
MANDIR ?= $(PREFIX)/share/man/man1
CARGO ?= cargo
VIEWER ?= 1

ifeq ($(VIEWER),0)
FEATURES := --no-default-features
endif

.PHONY: all build web test install uninstall clean

all: build

web:
	sh scripts/build-web.sh

build: web
	$(CARGO) build --release -p slml-cli $(FEATURES)

test:
	$(CARGO) test --all

install: build
	install -d "$(DESTDIR)$(BINDIR)" "$(DESTDIR)$(MANDIR)"
	install -m 755 target/release/slml "$(DESTDIR)$(BINDIR)/slml"
	install -m 644 docs/slml.1 "$(DESTDIR)$(MANDIR)/slml.1"
	@echo "installed slml to $(DESTDIR)$(BINDIR)/slml"

uninstall:
	rm -f "$(DESTDIR)$(BINDIR)/slml" "$(DESTDIR)$(MANDIR)/slml.1"

clean:
	$(CARGO) clean
