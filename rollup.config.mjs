import rust from '@wasm-tool/rollup-plugin-rust';
import url from '@rollup/plugin-url';

const deploy = !!process.env.DEPLOY;

export default {
    input: "./js/main.js",
    output: {
        dir: 'dist/js/',
    },
    plugins:[
        rust({
            serverPath: deploy ? "/asteroid-colonies/js/" : "./js/",
        }),
		url(),
    ]
}
