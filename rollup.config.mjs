import rust from '@wasm-tool/rollup-plugin-rust';
import url from '@rollup/plugin-url';

export default {
    input: "./js/main.js",
    output: {
        dir: 'dist/js/',
    },
    plugins:[
        rust({
            serverPath: "/js/",
        }),
		url(),
    ]
}
