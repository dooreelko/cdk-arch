`component` command should 
- essentially concentrate on Function nodes, their placement and relations
- include the system and container nodes
- exclude infrastructure nodes as they belong to `deployment`
- bug. it should correctly filter the nodes given a --container option (still including parents)
