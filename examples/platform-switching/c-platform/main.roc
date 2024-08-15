platform "echo-in-c"
    requires {} { main : { x : U8, y : U8 } }
    exposes []
    packages {}
    imports []
    provides [mainForHost]

mainForHost : { x : U8, y : U8 }
mainForHost = main
