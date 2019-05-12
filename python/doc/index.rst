deltachat python bindings
=========================

The ``deltachat`` Python package provides two bindings for the core C-library
of the https://delta.chat messaging ecosystem:

- :doc:`capi` is a lowlevel CFFI-binding to the
  `deltachat-core C-API <https://c.delta.chat>`_.

- :doc:`api` [work-in-progress] is a high level interface to deltachat-core which aims
  to be memory safe and thoroughly tested through continous tox/pytest runs.


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

