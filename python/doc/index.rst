deltachat python bindings
=========================

The ``deltachat`` Python package provides two bindings for the core Rust-library
of the https://delta.chat messaging ecosystem:

- :doc:`api` is a high level interface to deltachat-core which aims
  to be memory safe and thoroughly tested through continous tox/pytest runs.

- :doc:`capi` is a lowlevel CFFI-binding to the previous
  `deltachat-core C-API <https://c.delta.chat>`_ (so far the Rust library
  replicates exactly the same C-level API).



getting started
---------------

.. toctree::
   :maxdepth: 2

   install
   examples

.. toctree::
   :hidden:

   links
   changelog
   api
   capi
   lapi

..
    Indices and tables
    ==================

    * :ref:`genindex`
    * :ref:`modindex`
    * :ref:`search`

