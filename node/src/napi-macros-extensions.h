#include <napi-macros.h>

#undef NAPI_STATUS_THROWS

#define NAPI_STATUS_THROWS(call) \
  if ((call) != napi_ok) { \
    napi_throw_error(env, NULL, #call " failed!"); \
  }

#define NAPI_DCN_CONTEXT() \
  dcn_context_t* dcn_context; \
  NAPI_STATUS_THROWS(napi_get_value_external(env, argv[0], (void**)&dcn_context)); \
  if (!dcn_context) { \
    const char* msg = "Provided dnc_context is null"; \
    NAPI_STATUS_THROWS(napi_throw_type_error(env, NULL, msg)); \
  } \
  if (!dcn_context->dc_context) { \
    const char* msg = "Provided dc_context is null, did you close the context or not open it?"; \
    NAPI_STATUS_THROWS(napi_throw_type_error(env, NULL, msg)); \
  }

#define NAPI_DCN_ACCOUNTS() \
  dcn_accounts_t* dcn_accounts; \
  NAPI_STATUS_THROWS(napi_get_value_external(env, argv[0], (void**)&dcn_accounts)); \
  if (!dcn_accounts) { \
    const char* msg = "Provided dnc_acounts is null"; \
    NAPI_STATUS_THROWS(napi_throw_type_error(env, NULL, msg)); \
  } \
  if (!dcn_accounts->dc_accounts) { \
    const char* msg = "Provided dc_accounts is null, did you unref the accounts object?"; \
    NAPI_STATUS_THROWS(napi_throw_type_error(env, NULL, msg)); \
  }


#define NAPI_DC_CHAT() \
  dc_chat_t* dc_chat; \
  NAPI_STATUS_THROWS(napi_get_value_external(env, argv[0], (void**)&dc_chat));

#define NAPI_DC_CHATLIST() \
  dc_chatlist_t* dc_chatlist; \
  NAPI_STATUS_THROWS(napi_get_value_external(env, argv[0], (void**)&dc_chatlist));

#define NAPI_DC_CONTACT() \
  dc_contact_t* dc_contact; \
  NAPI_STATUS_THROWS(napi_get_value_external(env, argv[0], (void**)&dc_contact));

#define NAPI_DC_LOT() \
  dc_lot_t* dc_lot; \
  NAPI_STATUS_THROWS(napi_get_value_external(env, argv[0], (void**)&dc_lot));

#define NAPI_DC_MSG() \
  dc_msg_t* dc_msg; \
  NAPI_STATUS_THROWS(napi_get_value_external(env, argv[0], (void**)&dc_msg));

#define NAPI_ARGV_DC_MSG(name, position) \
  dc_msg_t* name; \
  NAPI_STATUS_THROWS(napi_get_value_external(env, argv[position], (void**)&name));

#define NAPI_DC_PROVIDER() \
  dc_provider_t* dc_provider; \
  NAPI_STATUS_THROWS(napi_get_value_external(env, argv[0], (void**)&dc_provider));

#define NAPI_DC_ARRAY() \
  dc_array_t* dc_array; \
  NAPI_STATUS_THROWS(napi_get_value_external(env, argv[0], (void**)&dc_array));

#define NAPI_RETURN_UNDEFINED() \
  return 0;

#define NAPI_RETURN_UINT64(name) \
  napi_value return_int64; \
  NAPI_STATUS_THROWS(napi_create_bigint_int64(env, name, &return_int64)); \
  return return_int64;

#define NAPI_RETURN_INT64(name) \
  napi_value return_int64; \
  NAPI_STATUS_THROWS(napi_create_int64(env, name, &return_int64)); \
  return return_int64;


#define NAPI_RETURN_AND_UNREF_STRING(name) \
  napi_value return_value; \
  if (name == NULL) { \
    NAPI_STATUS_THROWS(napi_get_null(env, &return_value)); \
    return return_value; \
  } \
  NAPI_STATUS_THROWS(napi_create_string_utf8(env, name, NAPI_AUTO_LENGTH, &return_value)); \
  dc_str_unref(name); \
  return return_value;

#define NAPI_ASYNC_CARRIER_BEGIN(name) \
  typedef struct name##_carrier_t { \
    napi_ref callback_ref; \
    napi_async_work async_work; \
    dcn_context_t* dcn_context;

#define NAPI_ASYNC_CARRIER_END(name) \
  } name##_carrier_t;

#define NAPI_ASYNC_EXECUTE(name) \
  static void name##_execute(napi_env env, void* data)

#define NAPI_ASYNC_GET_CARRIER(name) \
  name##_carrier_t* carrier = (name##_carrier_t*)data;

#define NAPI_ASYNC_COMPLETE(name) \
  static void name##_complete(napi_env env, napi_status status, void* data)

#define NAPI_ASYNC_CALL_AND_DELETE_CB() \
  napi_value global; \
  NAPI_STATUS_THROWS(napi_get_global(env, &global)); \
  napi_value callback; \
  NAPI_STATUS_THROWS(napi_get_reference_value(env, carrier->callback_ref, &callback)); \
  NAPI_STATUS_THROWS(napi_call_function(env, global, callback, argc, argv, NULL)); \
  NAPI_STATUS_THROWS(napi_delete_reference(env, carrier->callback_ref)); \
  NAPI_STATUS_THROWS(napi_delete_async_work(env, carrier->async_work));

#define NAPI_ASYNC_NEW_CARRIER(name) \
  name##_carrier_t* carrier = calloc(1, sizeof(name##_carrier_t)); \
  carrier->dcn_context = dcn_context;

#define NAPI_ASYNC_QUEUE_WORK(name, cb) \
  napi_value callback = cb; \
  napi_value async_resource_name; \
  NAPI_STATUS_THROWS(napi_create_reference(env, callback, 1, &carrier->callback_ref)); \
  NAPI_STATUS_THROWS(napi_create_string_utf8(env, #name "_callback", \
                                             NAPI_AUTO_LENGTH, \
                                             &async_resource_name)); \
  NAPI_STATUS_THROWS(napi_create_async_work(env, callback, async_resource_name, \
                                            name##_execute, name##_complete, \
                                            carrier, &carrier->async_work)); \
  NAPI_STATUS_THROWS(napi_queue_async_work(env, carrier->async_work));

/***  this could/should be moved to napi-macros ***/

#define NAPI_DOUBLE(name, val) \
  double name; \
  if (napi_get_value_double(env, val, &name) != napi_ok) { \
    napi_throw_error(env, "EINVAL", "Expected double"); \
    return NULL; \
  }

#define NAPI_ARGV_DOUBLE(name, i) \
  NAPI_DOUBLE(name, argv[i])
