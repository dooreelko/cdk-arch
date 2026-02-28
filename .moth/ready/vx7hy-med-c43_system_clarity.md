when generating `system` level of c4 description, the output should only 
inclide system components defining Architecture and its final consumers. 
e.g. in case of reveal-interact, the system-level nodes are the 
Architecture itself (it defines the entire backend), the plugin (it 
interacts with the backend) and the web/example (it interacts with the backend via revint-lib)

The infra modules (infra-cloudflare and infra-docker) are relevant at the deployment level
The e2e-tests are never deployed, so not interesting at any level
The revint-lib is a transient user of the api, so also must be excluded.

c43 should identify all those cases and filter out what's not required.
also this should be reflected in the `type` attribute of the node. 
An api provider is "backend", an obvious web site should be "frontend", 
the other api consumers are "client".
