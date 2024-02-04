import rust from '@wasm-tool/rollup-plugin-rust'

export default {
    input: "./src/main.js",
    output: {
        dir: 'dist/js/',
    },
    plugins:[
        rust({
            serverPath: "/js/",
        }),
    ]
}
