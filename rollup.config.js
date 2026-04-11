import typescript from '@rollup/plugin-typescript'

export default {
  input: 'guest-js/index.ts',
  output: [
    { file: 'dist-js/index.js', format: 'esm', sourcemap: true },
    { file: 'dist-js/index.cjs', format: 'cjs', sourcemap: true },
  ],
  external: ['@tauri-apps/api', '@tauri-apps/api/core', '@tauri-apps/api/event'],
  plugins: [typescript({ tsconfig: './tsconfig.json' })],
}
