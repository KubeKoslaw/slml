# Installing the compiler

SLML is one program, `slml`. It opens pages in SLML's own native browser and checks them for mistakes. You do not need Rust or any other tool to use it.

## Prebuilt compiler

**Linux and macOS:** run the install script. It puts `slml` into `~/.local/bin`.

```sh
curl -fsSL https://raw.githubusercontent.com/KubeKoslaw/slml/main/install.sh | sh
```

**Windows** (PowerShell):

```slml
irm https://raw.githubusercontent.com/KubeKoslaw/slml/main/install.ps1 | iex
```

You can also download an archive for your system from the [releases page](https://github.com/KubeKoslaw/slml/releases) and put the `slml` program anywhere on your `PATH`.

**Private repository**

While the SLML repository is private, downloads need a GitHub login. Install the GitHub CLI, run `gh auth login`, and the install script will download through it.

## Building from source

Install [Rust](https://rustup.rs) 1.88 or newer, then build and install the native browser:

```slml
git clone https://github.com/KubeKoslaw/slml
cd slml
make install
```

`make install` copies `slml` to `~/.local/bin` and its manual page to `~/.local/share/man`. For a system-wide install use `sudo make install PREFIX=/usr/local`. On a server without a graphical desktop, add `VIEWER=0` to leave out the native viewer window.

## Check that it works

```sh
slml --version
slml --help
```

You should see the compiler version and the language version, for example:

```slml
slml 0.1.0 (language 0.1)
```

## Editor setup

Any text editor works. Save files as UTF-8 with the `.slml` extension. Run `slml` with your site folder, or run `slml index.slml`: SLML's native browser reloads every time you save.

[Introduction](index.md)

[Your first page](first-page.md)

[SLML source](../../handbook/install.slml)
