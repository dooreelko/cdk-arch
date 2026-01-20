we need to restructure the code to have smallest-possible runtime packages

- create packages/ directory and move cdk-arch and example under it
- split example into separate packages:
	- architecture
	- local-docker
	- cloudflare

for cloudflare return to use cdk-arch, cloudflare's workers 
have crypto polyfill: https://developers.cloudflare.com/workers/runtime-apis/nodejs/crypto/

