# install-from

[English](README.md)

「このコマンドはどこから来たのか？」という一つの問いに答える macOS 向け CLI です。
すなわち、そのバイナリをディスク上に配置したパッケージマネージャ／仕組みを特定します。

```
$ install-from jq
jq: homebrew formula jq 1.7.1

$ install-from git
git: macos-system (shell command)

$ install-from node
node: version-manager asdf shim

$ install-from black
black: pip (python: /usr/local/opt/python@3.12/bin/python3.12)
```

## 背景

長く使い続けている macOS では、同じコマンド名でも Homebrew、バージョンマネージャ
（asdf/mise/nvm/pyenv/rbenv/volta）、npm/yarn/pnpm、uv、pipx、RubyGems、cargo、
go install、pip スクリプト、Xcode Command Line Tools、あるいは OS 自体など、
様々な経路でインストールされ得ます。`install-from` はディスク上のファイル
（配置場所とシンボリックリンクのチェーン）を調べるだけで判定し、サブプロセスは
一切起動しません。

## インストール

```bash
cargo install install-from
```

## 使い方

```
install-from [OPTIONS] <COMMAND>

Arguments:
  <COMMAND>   コマンド名（$PATH から検索）または '/' を含むパス

Options:
  --all       $PATH 上の最初の一致だけでなく、すべての一致を評価する
  --detail    結果行の下にパス / シンボリックリンクのチェーン / shebang を表示
  --json      JSON を出力する（detail フィールドを常に含む）
  -h, --help
  -V, --version
```

### 例

```
$ install-from jq --detail
jq: homebrew formula jq 1.7.1
  path:    /usr/local/bin/jq
  symlink: /usr/local/bin/jq -> ../Cellar/jq/1.7.1/bin/jq -> /usr/local/Cellar/jq/1.7.1/bin/jq
  shebang: (binary)

$ install-from jq --json
{
  "name": "jq",
  "path": "/usr/local/bin/jq",
  "resolved_path": "/usr/local/Cellar/jq/1.7.1/bin/jq",
  "symlink_chain": ["/usr/local/bin/jq", "../Cellar/jq/1.7.1/bin/jq"],
  "dangling": false,
  "shebang": null,
  "source": "homebrew",
  "package": "jq",
  "version": "1.7.1",
  "detail": { "kind": "formula", "prefix": "/usr/local" }
}
```

### 終了コード

| コード | 意味 |
| ---: | --- |
| 0 | インストール元を特定できた |
| 1 | ファイルは存在するが特定できなかった（`unknown`）／シンボリックリンクのループ |
| 2 | コマンドが見つからない／パスが存在しない／ディレクトリである／読み取り不可 |
| 3 | 使い方・引数のエラー |

## 検出対象（優先順位順）

シンボリックリンクのチェーンの各ホップに対して、優先順位の高い順に照合し、
最初に一致したものが採用されます。

1. `macos-system` — OS 同梱のバイナリ（`/bin`, `/sbin`, `/usr/bin`, `/usr/sbin`）
2. `xcode-clt` — Xcode / Command Line Tools
3. `version-manager` — asdf / mise / nvm / pyenv / rbenv / volta の shim
4. `homebrew` — formula または cask
5. `npm` — npm / yarn / pnpm によるグローバルインストール
6. `uv` — `uv tool install`
7. `pipx`
8. `gem` — RubyGems
9. `cargo` — `cargo install`
10. `go` — `go install`
11. `pip` — shebang が Python インタプリタであるスクリプト
12. `unknown` — どれにも一致しなかった

各検出器の詳細なフィールド仕様・エッジケース・照合ルールは
[SPEC.md](SPEC.md)（英語）を参照してください。

## 対象範囲

- 対応プラットフォーム: macOS のみ（Intel / Apple Silicon）。
- 外部コマンドは一切起動しません（`stat` / `readlink` / ファイル先頭行の読み取りのみ）。
- パッケージ名・バージョンは**パスの構成要素からのみ**導出し、パッケージの
  メタデータファイルは参照しません。
- Linux/Windows 対応、`.pkg` のレシート、コード署名情報、バージョンマネージャ
  の shim から実体バイナリへの解決などは現時点では対象外です
  （[SPEC.md §10](SPEC.md#10-future-not-mvp) 参照）。

## 開発

```bash
make build   # cargo build
make check   # fmt --check, clippy -D warnings, test（pre-push チェック）
make release # cargo build --release
```

CI は GitHub Actions の `macos-latest` 上で実行されます:
`cargo fmt --check`、`cargo clippy -- -D warnings`、`cargo test`。

## ライセンス

MIT OR Apache-2.0
