Delta Chat Python bindings, new and old
=======

`Delta Chat <https://delta.chat/>`_ provides two kinds of Python bindings
to the `Rust Core <https://github.com/deltachat/deltachat-core-rust>`_:
JSON-RPC bindings and CFFI bindings.
When starting a new project it is recommended to use JSON-RPC bindings,
which are used in the Delta Chat Desktop app through generated Typescript-bindings. 
The Python JSON-RPC bindings are maintained by Delta Chat core developers. 
Most existing bot projects and many tests in Delta Chat's own core library
still use the CFFI-bindings, and it is going to be maintained certainly also in 2024. 
New APIs might however only appear in the JSON-RPC bindings, 
as the CFFI bindings are increasingly in maintenance-only mode. 

.. toctree::
   :maxdepth: 2
   :caption: JSON-RPC Bindings

   jsonrpc/intro
   jsonrpc/install
   jsonrpc/examples
   jsonrpc/reference
   jsonrpc/develop

.. toctree::
   :maxdepth: 2
   :caption: CFFI Bindings

   cffi/intro
   cffi/install
   cffi/examples
   cffi/manylinux
   cffi/tests
   cffi/api
   cffi/lapi
   cffi/plugins

.. _`deltachat`: https://delta.chat
.. _`deltachat-core repo`: https://github.com/deltachat
.. _pip: http://pypi.org/project/pip/
.. _virtualenv: http://pypi.org/project/virtualenv/
.. _merlinux: http://merlinux.eu
.. _pypi: http://pypi.org/
.. _`issue-tracker`: https://github.com/deltachat/deltachat-core-rust
