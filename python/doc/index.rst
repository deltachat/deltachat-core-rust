deltachat python bindings
=========================

The ``deltachat`` Python package provides two layers of bindings for the
core Rust-library of the https://delta.chat messaging ecosystem:

- :doc:`api` is a high level interface to deltachat-core which aims
  to be memory safe and thoroughly tested through continous tox/pytest runs.

- :doc:`lapi` is a lowlevel CFFI-binding to the `Rust Core
  <https://github.com/deltachat/deltachat-core-rust>`_.



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
   lapi

..
    Indices and tables
    ==================

    * :ref:`genindex`
    * :ref:`modindex`
    * :ref:`search`

