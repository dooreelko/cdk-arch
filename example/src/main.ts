import {synth_cloudflare} from './cloudflare/terraform';
import {synth_local_docker} from './docker/terraform'


const synthModules: Record<string, () => void> = {
  docker: synth_local_docker,
  cloudflare: synth_cloudflare
};

const platform = process.env.PLATFORM || 'docker';

const synth = synthModules[platform];
if (synth) {
  synth();
} else {
  console.error(`Unknown platform: ${platform}. Supported: ${Object.keys(synthModules).join(', ')}`);
  process.exit(1);
}

export {};