app [makeGlue] { pf: platform "../platform/main.roc" }

import pf.Types exposing [Types]
import pf.File exposing [File]
import "../../compiler/builtins/bitcode/src/list.zig" as rocStdList : Str
import "../../compiler/builtins/bitcode/src/str.zig" as rocStdStr : Str
import "../../compiler/builtins/bitcode/src/utils.zig" as rocStdUtils : Str

makeGlue : List Types -> Result (List File) Str
makeGlue = \typesByArch ->
    typesByArch
    |> List.map convertTypesToFile
    |> List.concat staticFiles
    |> Ok

## These are always included, and don't depend on the specifics of the app.
staticFiles : List File
staticFiles = [
    { name: "list.zig", content: rocStdList },
    { name: "str.zig", content: rocStdStr },
    { name: "utils.zig", content: rocStdUtils },
]

convertTypesToFile : Types -> File
convertTypesToFile = \_ -> { name: "main.zig", content }

content =
    """
    // ⚠️ GENERATED CODE ⚠️ 
    //
    // This package is generated by the `roc glue` CLI command
    """
