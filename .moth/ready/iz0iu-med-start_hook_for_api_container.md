some servers need to perform actions on startup. 
thus we should extend ApiContainer to have a onStart() method that can be overloaded by children.

concrete implementations will carry the responsibility to call this hook.

additionally, the ApiContainer should convert the last parameter to be an object containing
- required routes 
- an optional no-args callback that will be called by onStart so that business logic can also plug into the event
