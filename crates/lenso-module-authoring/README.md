# lenso-module-authoring

`lenso-module-authoring` owns runtime-neutral authoring primitives shared by
Lenso Module frontends. It does not discover providers or mutate an App graph:
a typed `Port<C>` connects only through the adapter-owned dependency view
implemented by its generated `CapabilityClient`.
