http bindings are prevalent and repetitive, so instead of calling

```
architectureBinding.bind(api, {
  ...endpoint,
  overloads: {
    hello: httpHandler(endpoint, api, 'hello'),
    hellos: httpHandler(endpoint, api, 'hellos')
  }
})
```

we need a helper function that will take api, endpoint and list of route names to override and return 
an object that for each overriden route will expose an async function with signature same as the handler of the route.

