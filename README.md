# pg_ivfflat

Try building an IVFFlat index AM for Postgres.

# Components

1. [ivfflat](./ivfflat/): An in-memory IVFFlat index implementation.
2. [pg_extension](./pg_extension/): A Postgres extension that:

   1. Brings a `Vector` type and needed distance functions. 
   2. Implements the index AM `ivfflat`