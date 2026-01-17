In the example we're missing Postgres

- add a container for it and a network to be used between the postgres and the api server
- move the api server docker code to a separate file, use express for the server
- JsonStore shoud extend ApiContainer and declare store and get as APIs
- for the function, actually use the code in helloFunction
- when a cdktf component is instantiated, it should be associated with its architectural counterpart
- since postgres will be running out of process, associating cdktf component with an architectural one, should update the route function
with a wrapper that will make an http call using service discovery from docker runtime
