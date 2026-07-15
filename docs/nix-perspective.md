<!--
SPDX-FileCopyrightText: 2026 Emily <hello@emily.moe>

SPDX-License-Identifier: BlueOak-1.0.0
-->

# autopen’s design from a Nix perspective

Systems like Secure Boot rely on runtime cryptographic verification of embedded
in‐band signatures. Existing solutions for NixOS like
[Lanzaboote](https://github.com/nix-community/lanzaboote) focus on doing signing
outside of Nix builds, which avoids tricky questions about reproducibility and
purity in the context of the Nix build model.

autopen has been designed to work differently: it lets you put signing inside
Nix builds themselves, while taking care to maintain the fundamental properties
of the Nix model, and without needing to change anything about existing Nix
implementations, Hydra, or the NixOS cache. This document will hopefully
convince you that this is both necessary for a scalable upstream solution and a
good idea in its own right!

**tl;dr/spoiler:** Private build inputs denoting object capabilities
representing access to a deterministic signing service for a given key fit
perfectly into the Nix model, give us all the properties we care about,
sacrifice nothing that we don’t already lack in practice and actively want to
throw away for this use case, and can be modelled today without any changes
using an extra sandbox path pointing to a Unix socket that gives access to key
operations based on file descriptor passing of store objects representing
signing keys and expressed in the Nix language as standard build dependencies.

## Why should signing happen in Nix builds?

Signing outside of Nix builds works fine when you only have a few top‐level
signed UEFI executables and you can do signing as part of the deployment
process. It’s good to minimize the number of signatures that you need, and this
solution works fine for end‐user systems that have signing keys accessible.
However, an upstream solution can’t punt on this: a user that doesn’t want to
manage their own signing keys needs to be able to get pre‐signed packages from
upstream, and the upstream infrastructure needs to build installer images that
feed those signed executables into further complex build logic.

Some use cases may require even deeper nesting: Microsoft’s Secure Boot signing
policy requires enforcing kernel lockdown, which by default requires signing
each individual kernel module; those modules then end up in both the runtime
system closure and in stage 1 initramfs images. In practice, we hope to avoid
the need for individual module signatures, but the general point still applies.
For example, people who want to declaratively configure NixOS virtual machines
that enforce Secure Boot need to integrate signed bootloader and kernel image
packages as part of the VMs in their system closure, regardless of whether
they’re using official or self‐managed keys.

Moving every layer of assembly that needs to care about signed packages would
make NixOS builds and deployment much more complex, requiring moving builds of
installer and VM images outside of Nix and Hydra and establishing out‐of‐band
solutions for distributing and obtaining signed executables for end‐user
systems, reimplementing existing logic externally just to work around
limitations of our tooling. It would also throw away the benefits of the current
end‐to‐end model where a NixOS configuration is described entirely by a Nix
package that can be built remotely, substituted from caches, copied around, and
reproduced, just like everything else we build.

If this was necessary to preserve the core properties of Nix, then that would be
an unfortunate fact of life that we’d just have to live with. But it’s not – we
can do better!

## Properties of Nix derivations

At first blush, integrating signing into Nix builds might seem like a bad idea –
access to a signing key feels impure, and signatures that only the key holder
can produce seems to be in conflict with reproducibility.

To show how it can work and why it’s a good idea, let’s get more specific about
the properties that “pure” and “reproducible” are gesturing at, so we can
compare autopen’s signing derivations to the ones we already have. We’ll use the
following taxonomy (inspired by the
[object‐capability](https://en.wikipedia.org/wiki/Object-capability_model)
literature, specifically
[a similar taxonomy from E](https://web.archive.org/web/20250918070132/http://www.erights.org/elang/same-ref.html)):

1. **Public source:** The derivation source code that builds outputs is publicly
   available.

   Everything in Nixpkgs is public source, but if you’re given a NAR built from
   a derivation without getting the derivation itself, then you don’t have this
   property.

2. **Public inputs:** The derivation encodes everything required to attempt to
   build it; it has no external dependencies, and anyone with the derivation can
   attempt to build it.

   In practice, this is a relative property: we assume that we have all the
   explicit Nix-level build inputs when considering this property, as otherwise
   no derivation with dependencies would qualify, and the derivation `system`
   and `requiredSystemFeatures` are explicit carve‐outs – you need an
   implementation of `aarch64-darwin` to be able to build a derivation with that
   system, which is not publicly available, and if you have an `aarch64-linux`
   machine that nonetheless doesn’t support KVM, then you’d have trouble
   building a derivation with the `kvm` required system feature. A derivation
   can also depend on system calls that aren’t available in the kernel version
   you’re running. The bootstrappable builds effort is an attempt to minimize
   this wiggle room.

3. **Deterministic:** The derivation produces the same outputs every time it’s
   successfully built in any context, and the build will always succeed or
   always fail so long as it has sufficient hardware resources and its
   environment is set up as it expects (e.g. build dependencies, the OS,
   external inputs like the site a FOD downloads from).

   “The same” offers wiggle room here: the ideal is that the outputs are
   bit‐identical every time, but a derivation with a trivial reproducibility
   issue like including a timestamp in the output can still produce a
   functionally‐equivalent package every time. Therefore, we can speak more
   generally of determinism modulo a given equivalence relation.

4. **Verifiable:** Anyone who has a derivation and a purported output built from
   it can check whether it’s a faithful output of the derivation.

We define **transparency** as the combination of public source and public
inputs, a derivation that anyone can try to build, although they may not succeed
(e.g. due to insufficient hardware resources), and may not get the same outputs
(if the derivation is not deterministic).

For **standard “input‐addressed” derivations**, here’s how these properties
check out:

1. **Public source:** Yes, if you have access to the derivation. Everything in
   the Hydra cache is public source, although in the absence of verifiability
   this depends on trust in the infrastructure.

2. **Public inputs:** Yes. The whole point of input‐addressed derivations is to
   have inputs controlled as hermetically as possible. As long as you have the
   build closure and an appropriate implementation of the derivation’s system,
   you can try to build it.

3. **Deterministic:** Ideally. Nix does not strictly enforce this, but it is
   desirable. Not every derivation in Nixpkgs is bit‐deterministic (yet!), but a
   derivation that fails to even be *functionality*‐deterministic by sometimes
   producing a result that’s not interchangeable with other builds is considered
   seriously buggy. Flaky builds are also considered problematic.

4. **Verifiable:** Yes, if deterministic. A purported output of a
   bit‐deterministic input‐addressed derivation can be verified by simply
   running the build independently and checking if its output is identical.

For non‐bit‐deterministic packages, verification modulo a given equivalence
relation is computable to the extent that the relation is (for instance,
“identical except for timestamp fields” or “produces equivalent results on a
test suite”).

For **fixed‐output derivations**, the story is a little different:

1. **Public source:** Yes; the derivation encodes the steps you need to run to
   attempt to build it, usually by invoking curl or Git. `requireFile` is
   arguably an exception.

2. **Public inputs:** No; a fixed‐output derivation usually requires access to
   the internet to build, and the output it produces depends on the current
   state of the internet. Having a copy of the derivation alone is not
   sufficient to attempt to build it, and some FODs may even require specific
   kinds of internet access – for example, if they download from services that
   require authentication or block certain IPs.

3. **Deterministic:** Yes. A FOD that produces incorrect output simply fails to
   build; any successful build is bit‐identical ignoring hash collisions.
   Transient network issues are comparable to hardware issues causing
   input‐addressed derivations to be flaky. Bit rot can cause a FOD to stop
   building successfully, but we model that as failing to provide the correct
   non‐public inputs.

4. **Verifiable:** Yes. Since a FOD encodes the exact hash of its output, we can
   simply check a purported output against the digest. Indeed, this is precisely
   why we don’t consider them a serious violation of the build model. It also
   means that even when we cannot build a FOD – because of lacking network
   access or bit rot – we can still check whether a purported output is correct.

   Note that this is distinct from verifying whether a FOD accurately claims a
   hash that matches its download source, which is external to the Nix build
   model and the root cause of attacks like FOD poisoning – checking that is
   still a very good idea, but is orthogonal to checking whether a purported
   output matches the hash pinned by the FOD.

We can see that FODs take a process that violates some of the properties we
might want – like downloading from the internet, which requires external inputs
to the build and can produce arbitrary results – and tame it by enforcing
determinism, requiring the derivation to commit to a property that constrains it
to only one possible output. Indeed, a FOD is in some sense more deterministic
than an input‐addressed derivation: when built successfully, it will always have
a single fixed output; any issues reproducing a FOD’s build output are down to
builder issues or external bit rot.

When we talk about reproducibility, our end goal is generally verifiability –
helping establish trust that built outputs are faithful to a source derivation –
and the other properties are downstream of that. If an input‐addressed
derivation is bit‐deterministic, then we can take advantage of transparency by
building it independently and checking the outputs we get are identical to other
purported outputs – the canonical case for reproducibility. Community
reproducibility efforts do this with Nixpkgs at scale. For FODs, we don’t need
to do that to achieve verifiability; we can check any output against its claimed
hash without doing any work, even without internet access. (In practice,
reproducibility efforts do so anyway, as it also helps to catch bit rot and FOD
poisoning, and letting Nix just build entire closures is the most convenient way
to check as long as you have internet access.)

With autopen, we get a new class of **signing derivations**, which specify a
message and a signing key and produce a raw detached signature. They behave as
follows:

1. **Public source:** Yes; every signing derivation invokes a packaged version
   of autopen, has the message being signed as a build input, and uniquely
   identifies the signing key to use.

2. **Public inputs:** No; the derivation requires access to the signing key to
   build. Someone on a desert island with Nix and a signing derivation can’t
   necessarily build its output.

   Of course, this is a necessary property for the security of the signing
   scheme; the whole point of cryptographic signing is for not everyone to have
   the private signing key, so that seeing a signature means a specific trusted
   agent decided to sign the message. We wouldn’t *want* to let people sign
   anything they want with official NixOS signing keys, so we have to give this
   property up.

3. **Deterministic:** Many signature schemes are bit‐deterministic, or can be
   operated in such a mode, and we plan for autopen to use deterministic
   signatures whenever possible. The `RSASSA-PKCS1-v1_5` algorithm currently
   used by Secure Boot in practice is fully deterministic. Since the only
   functional purpose of a signature is to verify it,
   *functionality*‐determinism is always guaranteed.

   As with FODs, this is automatically enforced: if a valid signature with the
   expected key cannot be obtained, the derivation cannot be successfully built.

4. **Verifiable:** Trivially, up to the property of “is a signature of the given
   message with the corresponding verification key”; anyone can check whether a
   purported signature is valid, and as signature verification produces a
   boolean result, any two valid signatures are equivalent.

   A fully‐deterministic signature scheme yields bit‐verifiable derivations;
   verification with signature schemes that have both deterministic and
   randomized modes requires trusting only that the deterministic mode was used.
   (In other words: you can construct your ECDSA or ML‐DSA signing software such
   that it always produces deterministic signatures, but a third party can’t
   prove that any given single signature was produced that way. This is
   extremely unlikely to be a meaningful problem in practice.)

We can see that signing derivations are comparable to FODs in terms of formal
properties, and are inherently more deterministic than input‐addressed
derivations. Where FODs pin “the output hashes to a specific digest” ahead of
time, signing derivations pin “the output is a signature of a specific message
under a specific key”. With a deterministic signature scheme, this is just as
good for verifiability.

We necessarily lose transparency by way of the signing key representing a
non‐public input – and unlike with FODs, access to a signing key is much more
tightly controlled than access to the internet sources packages in Nixpkgs tend
to download from. However, as we’ll see later, we can still achieve a similar
“verify by re‐running the build” flow in practice, and even accommodate
independent builds on airgapped infrastructure. In practice, you’re always
strictly better off than with an input‐addressed derivation that requires more
RAM than you have, or for an OS you don’t have access to.

Therefore, autopen integrates signing into Nix derivations in a way that does
not violate any of the most essential properties of the Nix model.

## Implementing signing derivations

In practice, autopen implements these signing derivations on top of
input‐addressed derivations.

To understand how this works, we can start with the simplest possible derivation
that does cryptographic signing in a Nix build: it takes the message to be
signed and the private signing key as build inputs, and produces the signature
as its output.

With a deterministic signature scheme, this can satisfy all of the mentioned
properties, and be as sound in the Nix build model as any derivation. What that
would lack if we shared entire build closures, of course, is security: if
everyone gets access to the signing key, then the signature isn’t worth much.

We could fix this by performing these builds on trusted infrastructure, and then
distributing the signing derivations and their outputs publicly without ever
sharing the signing key store object itself. If the signing derivation pins the
verification key, then we get the expected properties: public source,
verifiable, as deterministic as the signature scheme, but not public inputs –
since nobody else has the signing key to import into the store, they can’t
actually build the derivation successfully.

This approach is in line with the object-capability model, but it has a few
problems. Firstly, Nix doesn’t like having derivations in the store that are
missing their inputs, and it can’t produce them as a result of evaluation (as
would be required to get seamless caching), even in impure evaluation mode. We
could work around this by using `requireFile`, making the signing key the
“output” of a public FOD that always fails to build, but even if we did that,
Hydra would happily copy the real thing to the cache as part of closures.

In practice, it would be difficult to stop the signing key from leaking; we
would have to teach Nix and Hydra about non‐world‐readable secret store objects
and private build inputs available only to certain derivations and not pushed
out with build‐time closures. Even if we did, the signing key would have to
reside on disk in the store, preventing us from using hardware‐backed keys. This
is a regression from Lanzaboote, bad for security, and potentially a serious
compliance problem.

Finally, although the derivations are verifiable in theory, in practice it would
require annoying manual work to verify them: we can’t simply try to build them
as we can with input‐addressed derivations and FODs, so we’d need custom tooling
to check purported outputs.

We can solve all of these problems at once by moving the private key material
out of the store. What would it take to have signing derivations that don’t have
access to raw key material? We’d need them to have build inputs that allow
signing messages with a given key without exposing the key itself. Unix sockets
allow exposing an interface to a protocol as a file while hiding the
implementation on the other end; if we could have Unix sockets as build inputs,
then our signing derivations could send the message to a Unix socket
representing access to a given signing key, get a signature back, verify it
against the expected verification key, and return it. The build sandbox would
ensure that builds cannot sign with any key they weren’t given a signing service
for as an input. As long as the service on the other end satisfies the expected
signing interface, we would preserve all the same properties as before, while
allowing the private key to be stored anywhere; it could be a software key
outside of the store, a hardware key, or a remote signing server. There’d also
be no risk of Nix exfiltrating private key material to the cache – a Unix socket
is inherently local and opaque.

This is exactly the conceptual object‐capability model autopen is based on – the
capability to use a signing key represented as unforgeable access to an opaque
signing service interface, in the form of a private build input – but as you
have probably noticed, implementing this by having Unix sockets directly in the
store would require some significant and fundamental changes to Nix.

Thankfully, we can do it without any changes at all! When using autopen for
signing in Nix builds, we set it up as a system service providing a Unix socket
accessible only to the Nix build users group, and expose it into the build
sandbox with `extra-sandbox-paths`. To implement the “signing capabilities as
build inputs” model and ensure that we maintain the purity of the sandbox, we
map each signing key to a unique store object, which acts as an opaque handle
for the key. We can construct those signing key handles in public Nix
expressions using the verification key, so that anyone can reproduce the signing
derivations without access to the signing key. To sign anything with a key,
clients have to pass a file descriptor for the corresponding key handle in the
store over the Unix socket, proving that they have access based on the
(`st_dev`, `st_ino`) file identity (we use directories as key handles to ensure
that `auto-optimise-store` doesn’t accidentally forge handles with hard links).
Clients that don’t have any key handles in their build closure can’t sign
anything, so access to the autopen socket is harmless and provides no new
capabilities – it’s conceptually equivalent to a system call they can’t use.

This gives us an equivalent model to the conceptual ideal today: signatures can
be pushed to the public cache and substituted by Nix users, Hydra can push full
Nix build closures out to the cache without leaking anything, hardware‐backed
keys can be used, and we can provide a path for easy verification and
reproduction of builds through configuration of the autopen daemon.

## Transparency logging and reproducing builds

Although public inputs is the one property we need to discard when we’re talking
about cryptographic signatures, there’s still some things about it we’d really
like to keep: even in a world where we have official NixOS signatures in
derivation outputs, it’s desirable for third parties to be able to build Nixpkgs
without using the cache! Even people who do use the official cache would like to
build packages, e.g. as part of
[reproducibility tracking](https://reproducibility.nixos.social/).

One solution is very simple – parameterizing Nixpkgs on keys, so that people can
build on their own infrastructure using their own signing keys. This can be done
with a simple overlay, and is the natural choice for people who want to run
builds fully independently without trusting Hydra. (We can also offer
publicly‐available software keys in Nixpkgs so that you can build everything
with the same deterministic test signatures as everyone else would get, if you
don’t care about security but want to build things that go through signing
derivations.)

That results in different derivations to the ones built by Hydra, though – since
the signing key is different, the hash of the signing derivations is too, and
that difference propagates all the way to the ISO. What if you want to build
everything from scratch on your own infrastructure to verify the reproducibility
of Hydra builds, while still using the exact same derivations referencing
official NixOS keys?

Thankfully, we can still achieve this! As a baseline, we ensure that the
derivations using autopen are as tightly scoped as possible: we never create
signatures within a derivation that does an actual build. A signing derivation
only ever takes payloads to sign as inputs and produces detached signatures with
a given key – even creating the payload to sign, and gluing the resulting
signature on to the executable, are done with standard tooling inside normal
derivations that anyone can build. This means that everything still works
exactly as before as long as you have the detached signatures, reducing the
problem to those.

That’s where tamper‐evident [transparency logs](https://transparency.dev/) come
in. Since we have a signing service, we can (once this is implemented in
autopen) publicly record every signature that gets built on Hydra in a
verifiable append‐only log. This is great for security and auditability, and has
been proposed as a
[potential future requirement](https://github.com/rhboot/shim-review/issues/291)
for Secure Boot. (Unix sockets also enable us to record which build produced a
given signature, and hopefully in the future we can attest the software
configuration of builders too.)

It also means that we can configure autopen to use a transparency log as a
signing backend even for keys we don’t possess. Whenever it receives a request
to sign a message, it can look up the hash in a transparency log, return the
logged signature if present after verifying it and its inclusion proof, and
error out otherwise – which is no different to the way that issues with network
access can prevent FOD builds from succeeding. When it succeeds, this produces
identical results to the real key, without having to trust the log. This does
require the payloads being signed to be bit‐deterministic, but that’s table
stakes for packages that are critical parts of the boot security chain.

Since anyone can mirror the public transparency log, nodes helping verify
reproducible builds can also act as independent
[cosigners](https://c2sp.org/tlog-cosignature@v1.0.1) or
[mirrors](https://c2sp.org/tlog-mirror), and builds reusing official signatures
can even be done on airgapped infrastructure. (You do need to keep the
transparency log synced when updating Nixpkgs, but you need some way of getting
things into your airgapped build infrastructure anyway – a new version of
Nixpkgs also means new FODs for sources.)

It’s true that this means you can only reproduce signatures that Hydra has
already produced, but that’s inherent. If you’re customizing things, then you
need to use your own keys anyway; if you don’t want to put trust in the official
Hydra infrastructure, then you’ll only want to trust your own keys regardless;
and if you’re trying to check the determinism of Hydra builds, then you can only
compare them once it finishes building anyway. As long as you follow the
official channels, the signatures you need for the builds will always be
available. Even if you run ahead of them, you can still build everything else in
the meantime – this only blocks the reverse closures of signing derivations,
which are ideally kept shallow wherever possible.

If a user ends up trying to build a signing derivation without having set this
up, we can arrange for autopen to fail gracefully, pointing them to
documentation on how to set up autopen with a transparency log backend, how to
configure custom keys, and how to use the insecure test keys.

## Securing signed packages in practice

Nixpkgs has a lot of packages and a lot of committers. It’s unavoidable that we
have to trust people who can merge changes to packages in the build closures of
things that get signed, but can we minimize the attack surface beyond that?

autopen’s design helps us out here:

* Since signing key handles are Nix store dependencies, we can track them with
  normal package dependencies; any signing derivation pulls the relevant key
  handle in as a direct input. We can use normal scoping rules to define the key
  handles only within the package set of trusted signed packages, and set
  appropriate code owners for its directory.

  Ideally, only Nix expressions that are explicitly given access to an opaque
  key handle would be able to use it, but as Nix lacks unforgeable capabilities,
  the Nix‐side library’s enforcement of this is best‐effort in practice; a
  malicious package expression could define the key handle store object inline,
  or pull it in through a recursive build closure using `.drvPath`. We could
  still defend against this by having a check to ensure that only the
  instantiated derivations in the signed package set reference official signing
  key handles in their inputs, although note that the compromised committer this
  attack would likely require can currently bypass the GitHub CI anyway, and
  transparency logs would make it detectable regardless.

* The Nix sandbox and autopen’s FD‐passing design ensure that derivations that
  aren’t declared to use any signing keys on the Nix level can’t access any. A
  compromised upstream build system running in a non‐signing derivation can’t
  sign anything, and signing derivations only use the standard build environment
  and autopen.

* Individual builder nodes don’t need direct access to signing keys, even
  hardware‐backed ones. They can be configured with an autopen socket whose
  backend forwards to a central locked‐down signing server that never runs any
  builds.

* Transparency logs and community reproducibility efforts allow public auditing
  of signatures, including automated third‐party verification that only
  signatures corresponding to the signed package set have been issued and that a
  from‐scratch build of the closures of the signed packages produces identical
  results.

(Note that transparency logging and signing over the network have not been
implemented yet at the time of writing.)

## Could autopen be used to handle the existing out‐of‐band NAR info signing on Hydra?

The software‐key NAR info package signing flow is definitely a weak link of the
current NixOS infrastructure, and it’s a design goal for autopen to potentially
be suitable for replacing the current Hydra signing flow in the future. Although
the integration with the Nix build model isn’t as relevant for this use case,
support for remote signing, attestation of builders, hardware‐backed keys, and
transparency logs would all help greatly improve the trustworthiness of the
official binary package signatures.

## Okay, you’ve convinced me. It’s amazing. You’re amazing. But I’m still thinking about what you said about object capabilities and private build inputs and non‐file store objects. What would it be like in a world where we actually had that stuff? Should we be doing all secrets management that way? Should we be doing service management with derivation outputs representing live service instances and modelling service sandboxing and dependencies that way? Is there secretly a better version of NixOS struggling to be born that sticks closer to the underlying Nix model by moving from pure functional programming to object capabilities?

Yes, precisely. I’m so glad you asked!
