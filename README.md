<img src="https://travis-ci.org/dnorman/unbase.svg?branch=master">

# What is Unbase?

Unbase is a concept for a distributed database/application framework that is fundamentally reactive, fault tolerant, and decentralized. It seeks to address some very specific shortcomings in traditional paradigms.

Unbase is an attempt to create a distributed architecture that transcends device, geography, programming language, and present orthodoxy about what constitutes a "database". It seeks to blur the lines between application/database, and client/server.

We argue that for many use cases, data should not be assigned to any specific storage location (as is the case in "sharded" systems) but rather, it should live where it originates, and where it's desired. Data should not be "based"[1] and thus the name: Unbase

*Unbase is presently under active development.*

## Design Goals:

* Zero-coordination / Zero waiting
* Drastically reduce operational latency by focusing on data locality (planet/continent/city/building/main memory/processor core)
* Provide tunable durability guarantees
* Reduce human planning and monetary cost through behavior-driven design
* Employ peer-to-peer networking to ensure continued operation during network partitions[2]
* Provide a robust type system, avoiding workarounds[3] commonly employed by RDBMS
* Common, minimalist library for client and server[4] applications
* Distributed content-filtered pubsub for efficient push notifications
* Provide a facility for the registration and execution of triggers to allow for reactive, but loose couplings

## Consistency Model

Unbase seeks to implement a specific causal consistency model which we are calling "Infectious Knowledge".
See [Consistency Model](CONSISTENCY_MODEL.md) for more details

[1] When data is "based" in a specific place (IE: consistent hashing) which is determined by anything other than the set of parties who are interested in its existence (originator/consumer) it is substantially more likely that a system will be compelled to wait for its retrieval. See [light cones](https://en.wikipedia.org/wiki/Light_cone)
[2] Using the term "partition" for conversational understanding. Partitions are not actually a thing.
[3] entity-attribute-value, serializations stored as text, etc
