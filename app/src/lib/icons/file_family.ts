// Path: app/src/lib/icons/file_family.ts
// Description: Extension-to-language-family mapping for file-type icon resolution

export type FileFamily =
  | "typescript"
  | "javascript"
  | "rust"
  | "python"
  | "go"
  | "java"
  | "csharp"
  | "cpp"
  | "ruby"
  | "swift"
  | "dart"
  | "elixir"
  | "clojure"
  | "lua"
  | "php"
  | "scala"
  | "html"
  | "css"
  | "vue"
  | "svelte"
  | "shell"
  | "sql"
  | "config"
  | "markup"
  | "markdown"
  | "r"
  | "julia"
  | "perl"
  | "elm"
  | "zig"
  | "nim"
  | "fsharp"
  | "solidity"
  | "verilog"
  | "astro"
  | "docker"
  | "makefile"
  | "generic";

const EXT_MAP: Map<string, FileFamily> = new Map([
  // TypeScript
  [".ts", "typescript"], [".tsx", "typescript"],
  // JavaScript
  [".js", "javascript"], [".jsx", "javascript"],
  [".cjs", "javascript"], [".mjs", "javascript"],
  // Rust
  [".rs", "rust"],
  // Python
  [".py", "python"], [".pyi", "python"],
  // Go
  [".go", "go"],
  // Java / JVM
  [".java", "java"], [".kt", "java"], [".kts", "java"],
  [".gradle", "java"], [".groovy", "java"],
  // C#
  [".cs", "csharp"],
  // C / C++ / Obj-C
  [".c", "cpp"], [".cc", "cpp"], [".cpp", "cpp"], [".cxx", "cpp"],
  [".h", "cpp"], [".hh", "cpp"], [".hpp", "cpp"], [".hxx", "cpp"],
  [".inl", "cpp"], [".ipp", "cpp"], [".mm", "cpp"], [".m", "cpp"],
  // Ruby
  [".rb", "ruby"], [".rake", "ruby"],
  // Swift
  [".swift", "swift"],
  // Dart
  [".dart", "dart"],
  // Elixir / Erlang
  [".ex", "elixir"], [".exs", "elixir"],
  [".erl", "elixir"], [".hrl", "elixir"],
  // Clojure
  [".clj", "clojure"], [".cljs", "clojure"],
  [".cljc", "clojure"], [".edn", "clojure"],
  // Lua
  [".lua", "lua"],
  // PHP
  [".php", "php"], [".phtml", "php"],
  // Scala
  [".scala", "scala"],
  // HTML
  [".html", "html"], [".htm", "html"], [".ejs", "html"],
  // CSS
  [".css", "css"], [".scss", "css"], [".sass", "css"], [".less", "css"],
  // Vue
  [".vue", "vue"],
  // Svelte
  [".svelte", "svelte"],
  // Shell
  [".sh", "shell"], [".bash", "shell"], [".zsh", "shell"],
  [".fish", "shell"], [".csh", "shell"], [".bat", "shell"],
  [".cmd", "shell"], [".ps1", "shell"], [".psd1", "shell"],
  [".psm1", "shell"], [".nu", "shell"],
  // SQL
  [".sql", "sql"],
  // Config
  [".json", "config"], [".json5", "config"], [".jsonc", "config"],
  [".yaml", "config"], [".yml", "config"], [".toml", "config"],
  [".ini", "config"], [".conf", "config"], [".properties", "config"],
  [".envrc", "config"], [".tf", "config"], [".tfvars", "config"],
  // Markup
  [".xml", "markup"], [".xsd", "markup"], [".xsl", "markup"],
  [".xslt", "markup"], [".graphql", "markup"], [".gql", "markup"],
  [".proto", "markup"], [".mdx", "markup"],
  // Markdown / text
  [".md", "markdown"], [".txt", "markdown"], [".rst", "markdown"],
  [".adoc", "markdown"], [".asciidoc", "markdown"], [".wiki", "markdown"],
  // R
  [".r", "r"],
  // Julia
  [".jl", "julia"],
  // Perl
  [".pl", "perl"], [".pm", "perl"],
  // Elm
  [".elm", "elm"],
  // Zig
  [".zig", "zig"],
  // Nim
  [".nim", "nim"],
  // F#
  [".fs", "fsharp"], [".fsi", "fsharp"],
  [".fsx", "fsharp"], [".fsharp", "fsharp"],
  // Solidity
  [".sol", "solidity"],
  // Verilog / VHDL
  [".v", "verilog"], [".sv", "verilog"], [".svh", "verilog"],
  [".vhd", "verilog"], [".vhdl", "verilog"],
  // Astro
  [".astro", "astro"],
  // Docker
  [".dockerfile", "docker"],
  // Makefile
  [".make", "makefile"],
]);

/** Extensionless filename → family */
const NAME_MAP: Map<string, FileFamily> = new Map([
  ["makefile", "makefile"],
  ["gnumakefile", "makefile"],
  ["dockerfile", "docker"],
  ["gemfile", "ruby"],
  ["rakefile", "ruby"],
  ["justfile", "makefile"],
  ["cmakelists.txt", "makefile"],
]);

export function getFileFamily(path: string): FileFamily {
  const segments = path.split("/");
  const filename = (segments[segments.length - 1] ?? "").toLowerCase();

  // Check extensionless names first
  const byName = NAME_MAP.get(filename);
  if (byName) return byName;

  // Extract extension
  const dotIdx = filename.lastIndexOf(".");
  if (dotIdx === -1) return "generic";

  const ext = filename.slice(dotIdx);
  return EXT_MAP.get(ext) ?? "generic";
}
