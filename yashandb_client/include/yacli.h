#ifndef YACLIC_H
#define YACLIC_H

#include <stdio.h>
#include <stdlib.h>
#include <stdint.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

/* YACLI option values */
#define YAC_TRUE true
#define YAC_FALSE false

/* YacConnAttr YAC_ATTR_PACKET_SIZE range */
#define YAC_MIN_PACKET_SIZE 65536    // 64KB
#define YAC_MAX_PACKET_SIZE 33554432 // 32MB

/* YACLI special length/indicator values */
#define YAC_NULL_DATA (-1)
#define YAC_NULL_TERM_STR (-2)

#define YAC_LOB_INVALID_CHARSET_ID (-1)

#define YAC_NUMBER_UNSIGNED 0
#define YAC_NUMBER_SIGNED 2

#define YAC_CALL(proc)                          \
    do {                                        \
        if ((YacResult)(proc) == YAC_ERROR) {   \
            return YAC_ERROR;                   \
        }                                       \
    } while (0)

#pragma pack(4)

/* YACLI declaration data types */
typedef int8_t         YacInt8;
typedef uint8_t        YacUint8;
typedef int16_t        YacInt16;
typedef uint16_t       YacUint16;
typedef int32_t        YacInt32;
typedef uint32_t       YacUint32;
typedef int64_t        YacInt64;
typedef uint64_t       YacUint64;
typedef char           YacChar;
typedef bool           YacBool;
typedef double         YacDouble;
typedef float          YacFloat;
typedef YacInt64       YacDate;
typedef YacInt64       YacShortTime;
typedef YacInt32       YacYMInterval;
typedef YacInt64       YacDSInterval;

#define YAC_NUMBER_SIZE 20
typedef struct StYacNumber {
    YacUint8 numberPart[YAC_NUMBER_SIZE];
} YacNumber;

#define YAC_ROWID_SIZE 16
typedef struct StYacRowId {
    YacUint8 rowIdPart[YAC_ROWID_SIZE];
} YacRowId;

#define YAC_TIMESTAMP_SIZE 12
typedef struct StYacTimestamp {
    YacUint8 timestampPart[YAC_TIMESTAMP_SIZE];
} YacTimestamp;

typedef struct StYacLobLocator YacLobLocator;
typedef struct StYacString     YacString;
typedef struct StYacRaw        YacRaw;

/* YACLI generic data structures */
typedef void     YacVoid;
typedef YacVoid* YacPointer;
typedef YacVoid* YacHandle;

/* YACLI error pos structure */
typedef struct StYacTextPos {
    YacInt32 line;
    YacInt32 column;
} YacTextPos;

#pragma pack()

/* YACLI return values from functions */
typedef enum EnYacResult {
    YAC_SUCCESS = 0,
    YAC_SUCCESS_WITH_INFO = 1,
    YAC_ERROR = -1,
} YacResult;

typedef enum EnYacRowStatus {
    YAC_ROW_SUCCESS = 0,
    YAC_ROW_SUCCESS_WITH_INFO = 1,
    YAC_ROW_NO_ROW = 2,
    YAC_ROW_ERROR = -1,
} YacRowStatus;

/* YACLI handle type codes */
typedef enum EnYacHandleType {
    YAC_HANDLE_UNKNOWN = 0,
    YAC_HANDLE_ENV = 1,
    YAC_HANDLE_DBC = 2,
    YAC_HANDLE_STMT = 3,
} YacHandleType;

/* YACLI internal data type codes, for describe column*/
typedef enum EnYacType {
    YAC_TYPE_UNKNOWN = 0,
    YAC_TYPE_BOOL = 1,
    YAC_TYPE_TINYINT = 2,
    YAC_TYPE_SMALLINT = 3,
    YAC_TYPE_INTEGER = 4,
    YAC_TYPE_BIGINT = 5,
    YAC_TYPE_UTINYINT = 6,
    YAC_TYPE_USMALLINT = 7,
    YAC_TYPE_UINTEGER = 8,
    YAC_TYPE_UBIGINT = 9,
    YAC_TYPE_FLOAT = 10,
    YAC_TYPE_DOUBLE = 11,
    YAC_TYPE_NUMBER = 12,
    YAC_TYPE_DATE = 13,
    YAC_TYPE_SHORTTIME = 15,
    YAC_TYPE_TIMESTAMP = 16,
    YAC_TYPE_TIMESTAMP_LTZ = 17,
    YAC_TYPE_TIMESTAMP_TZ = 18,
    YAC_TYPE_YM_INTERVAL = 19,
    YAC_TYPE_DS_INTERVAL = 20,
    YAC_TYPE_CHAR = 24,
    YAC_TYPE_NCHAR = 25,
    YAC_TYPE_VARCHAR = 26,
    YAC_TYPE_NVARCHAR = 27,
    YAC_TYPE_BINARY = 28,
    YAC_TYPE_CLOB = 29,
    YAC_TYPE_BLOB = 30,
    YAC_TYPE_BIT = 31,
    YAC_TYPE_ROWID = 32,
    YAC_TYPE_NCLOB = 33,
    YAC_TYPE_CURSOR = 34,
    YAC_TYPE_JSON = 35,
    YAC_TYPE_XML = 39,
    YAC_TYPE_NUMERIC_FLOAT = 40,
} YacType;

/* YACLI external data type codes, for bind parameter or bind column*/
typedef enum EnYacExtType {
    YAC_SQLT_UNKNOWN = 0,
    YAC_SQLT_BOOL = 1,
    YAC_SQLT_TINYINT = 2,
    YAC_SQLT_SMALLINT = 3,
    YAC_SQLT_INTEGER = 4,
    YAC_SQLT_BIGINT = 5,
    YAC_SQLT_UTINYINT = 6,
    YAC_SQLT_USMALLINT = 7,
    YAC_SQLT_UINTEGER = 8,
    YAC_SQLT_UBIGINT = 9,
    YAC_SQLT_FLOAT = 10,
    YAC_SQLT_DOUBLE = 11,
    YAC_SQLT_NUMBER = 12,
    YAC_SQLT_DATE = 13,
    YAC_SQLT_SHORTTIME = 15,
    YAC_SQLT_TIMESTAMP = 16,
    YAC_SQLT_TIMESTAMP_LTZ = 17,
    YAC_SQLT_TIMESTAMP_TZ = 18,
    YAC_SQLT_YM_INTERVAL = 19,
    YAC_SQLT_DS_INTERVAL = 20,
    YAC_SQLT_CHAR = 24,
    YAC_SQLT_VARCHAR = 26,
    YAC_SQLT_BINARY = 28,
    YAC_SQLT_CLOB = 29,
    YAC_SQLT_BLOB = 30,
    YAC_SQLT_BIT = 31,
    YAC_SQLT_ROWID = 32,
    YAC_SQLT_NCLOB = 33,
    YAC_SQLT_CURSOR = 34,
    YAC_SQLT_JSON = 35,
    YAC_SQLT_XML = 39,

    // diff with YAC_SQLT_CHARYAC_SQLT_VARCHAR and YAC_SQLT_BINARY in bind parameter input param,
    // step size in multi bind is bufLen, same with OCI and ODBC
    YAC_SQLT_CHAR2 = 100,
    YAC_SQLT_VARCHAR2 = 101,
    YAC_SQLT_BINARY2 = 102,

    YAC_SQLT_OBJSTRING = 103,
    YAC_SQLT_OBJRAW = 104,

    YAC_SQLT_NVARCHAR2 = 105,
} YacExtType;

/* YACLI bind direction for yacBindParameter and yacBindParameterByName*/
typedef enum EnYacParamDirection {
    YAC_PARAM_INPUT = 1,
    YAC_PARAM_OUTPUT = 2,
    YAC_PARAM_INOUT = 3,
} YacParamDirection;

/* YACLI env attr CHARSET_CODE options*/
typedef enum EnYacCharsetCode {
    YAC_CHARSET_ASCII = 0,
    YAC_CHARSET_GBK = 1,
    YAC_CHARSET_UTF8 = 2,
    YAC_CHARSET_ISO88591 = 3,
    YAC_CHARSET_UTF16 = 4,
    YAC_CHARSET_GB18030 = 5,
} YacCharsetCode;

/* YACLI env attrs */
typedef enum EnYacEnvAttr {
    YAC_ATTR_CHARSET_CODE = 62,
    YAC_ATTR_RETURN_SUCCESS_WITH_INFO = 64,
    YAC_ATTR_SOFTWARE_VERSION = 65,
    YAC_ATTR_CLIENT_DRIVER = 66,
} YacEnvAttr;

/* YACLI conn attrs */
typedef enum EnYacConnAttr {
    YAC_ATTR_AUTOCOMMIT = 3,
    YAC_ATTR_LOGIN_TIMEOUT = 4,
    YAC_ATTR_PACKET_SIZE = 6,
    YAC_ATTR_TXN_ISOLATION = 7,
    YAC_ATTR_CREDT = 11,
    YAC_ATTR_MAX_CHARSET_RATIO = 12,
    YAC_ATTR_TAF_ENABLED = 14,
    YAC_ATTR_TAF_CALLBACK = 15,
    YAC_ATTR_MAX_NCHARSET_RATIO = 17,
    YAC_ATTR_HEARTBEAT_ENABLED = 18,
} YacConnAttr;

/* YACLI conn attr TXN_ISOLATION options*/
typedef enum EnYacTxnIsolation {
    YAC_TXN_READ_COMMITTED = 0,
    YAC_TXN_CURR_COMMITTED = 1,
    YAC_TXN_SERIALIZABLE = 2,
} YacTxnIsolation;

/* YACLI stmt attrs */
typedef enum EnYacStmtAttr {
    YAC_ATTR_PARAMSET_SIZE = 100,
    YAC_ATTR_ROWSET_SIZE = 101,
    YAC_ATTR_ROWS_FETECHED = 102,
    YAC_ATTR_ROWS_AFFECTED = 103,
    YAC_ATTR_CURSOR_EOF = 104,
    YAC_ATTR_SQLTYPE = 105,
    YAC_ATTR_IMPLICIT_RESULT_COUNT = 113,
    YAC_ATTR_GET_DATA_SUPPORT = 114,
    YAC_ATTR_ROWS_STATUS = 117,
    YAC_ATTR_TIMEOUT = 120,
} YacStmtAttr;

/* YACLI conn attr CREDT options*/
typedef enum StYacCredtType {
    YAC_CRED_RDBMS = 0,
    YAC_CRED_EXT = 1,
} YacCredtType;

/* YACLI yacColAttribute attrs */
typedef enum EnYacColAttr {
    YAC_COL_ATTR_DISPLAY_SIZE = 0,
    YAC_COL_ATTR_NAME = 1,
    YAC_COL_ATTR_SIZE = 2,
    YAC_COL_ATTR_TYPE = 3,
    YAC_COL_ATTR_PRECISION = 4,
    YAC_COL_ATTR_SCALE = 5,
    YAC_COL_ATTR_NULLABLE = 6,
    YAC_COL_ATTR_CHAR_SIZE = 7,
    YAC_COL_ATTR_CHAR_USED = 8,
    YAC_COL_ATTR_DISPLAY_CHAR_SIZE = 9,
} YacColAttr;

/* YACLI yacStmtGetNextResult type */
typedef enum EnYacResultType {
    YAC_RESULT_TYPE_SELECT = 0,
} YacResultType;

/* YACLI stmt attr YAC_ATTR_SQLTYPE options */
typedef enum EnYacSQLType {
    YAC_SQLTYPE_UNKNOWN = 0,
    YAC_SQLTYPE_QUERY = 1,
    YAC_SQLTYPE_INSERT = 2,
    YAC_SQLTYPE_UPDATE = 3,
    YAC_SQLTYPE_DELETE = 4,
    YAC_SQLTYPE_MERGE = 5,
    YAC_SQLTYPE_CREATE = 11,
    YAC_SQLTYPE_ALTER = 30,
    YAC_SQLTYPE_DROP = 38,
    YAC_SQLTYPE_GRANT = 69,
    YAC_SQLTYPE_REVOKE = 70,
    YAC_SQLTYPE_COMMIT = 129,
    YAC_SQLTYPE_ROLLBACK = 130,
} YacSQLType;

typedef enum EnYacTempLobType {
    YAC_TEMP_BLOB  = 0,
    YAC_TEMP_CLOB  = 1,
    YAC_TEMP_NCLOB = 2,
} YacTempLobType;

/* YACLI TAF CallBackSet */
typedef enum EnYacTafResult {
    YAC_TAF_SUCCESS = 0,
    YAC_TAF_RETRY = 25410,
    YAC_TAF_ERROR = -1,
} YacTafResult;

typedef enum EnYacTafType {
    YAC_TAF_TYPE_NONE = 0,
    YAC_TAF_TYPE_SESSION = 1,
    YAC_TAF_TYPE_SELECT = 2,
} YacTafType;

typedef enum EnYacTafEvent {
    YAC_TAF_EVENT_BEGIN = 0,
    YAC_TAF_EVENT_END = 1,
    YAC_TAF_EVENT_ABORT = 2,
    YAC_TAF_EVENT_ERROR = 3,
} YacTafEvent;

typedef YacTafResult (*YacTafCallback)(YacHandle hConn, YacHandle hEnv, YacPointer tafCtx, YacTafType tafType, YacTafEvent tafEvent);

typedef struct {
    YacTafCallback tafCallbackFunc;
    YacPointer     tafCtx;
} YacTafCallbackStruct;

/* YACLI mem callbacks */
typedef YacPointer (*YacMalocFunc)(YacPointer ctxp, size_t size);
typedef YacPointer (*YacRalocFunc)(YacPointer ctxp, YacPointer memptr, size_t newSize);
typedef YacVoid (*YacMfreeFunc)(YacPointer ctxp, YacPointer memptr);

/* YACLI get error infomations APIs */
YacResult yacGetDiagRec(YacInt32* errCode, YacChar* message, YacInt32 bufLen, YacInt32* incicator, YacChar* sqlState, YacInt32 sqlStateBufLen, YacTextPos* pos);

/* YACLI handle alloc and free APIs */
YacResult yacAllocHandle(YacHandleType type, YacHandle input, YacHandle* output);
YacResult yacAllocEnvWithMemCb(YacHandle* env, YacPointer ctxp, YacMalocFunc malocFp, YacRalocFunc ralocFp, YacMfreeFunc mfreeFp);
YacResult yacFreeHandle(YacHandleType type, YacHandle handle);

/* YACLI conn APIs */
YacResult yacConnect(YacHandle hConn, const YacChar* url, YacInt16 urlLen, const YacChar* user, YacInt16 userLen, const YacChar* pwd, YacInt16 pwdLen);
YacVoid   yacDisconnect(YacHandle hConn);

/* YACLI prepare/execute APIs */
YacResult yacDirectExecute(YacHandle hStmt, const YacChar* sql, YacInt32 sqlLength);
YacResult yacPrepare(YacHandle hStmt, const YacChar* sql, YacInt32 sqlLength);
YacResult yacExecute(YacHandle hStmt);

/* YACLI fetch APIs */
YacResult yacFetch(YacHandle hStmt, YacUint32* rows);

/* YACLI end transation APIs */
YacResult yacCommit(YacHandle hConn);
YacResult yacRollback(YacHandle hConn);

/* YACLI cancel APIs */
YacResult yacCancel(YacHandle hConn);

/* YACLI set/get attrs APIs */
YacResult yacSetEnvAttr(YacHandle hEnv, YacEnvAttr attr, YacVoid* value, YacInt32 length);
YacResult yacGetEnvAttr(YacHandle hEnv, YacEnvAttr attr, YacVoid* value, YacInt32 bufLength, YacInt32* stringLength);
YacResult yacSetConnAttr(YacHandle hConn, YacConnAttr attr, YacVoid* value, YacInt32 length);
YacResult yacGetConnAttr(YacHandle hConn, YacConnAttr attr, YacVoid* value, YacInt32 bufLength, YacInt32* stringLength);
YacResult yacSetStmtAttr(YacHandle hStmt, YacStmtAttr attr, YacVoid* value, YacInt32 length);
YacResult yacGetStmtAttr(YacHandle hStmt, YacStmtAttr attr, YacVoid* value, YacInt32 bufLength, YacInt32* stringLength);

/* YACLI bind APIs */
YacResult yacBindColumn(YacHandle hStmt, YacUint16 id, YacUint32 extType, YacPointer value, YacInt32 bufLen, YacInt32* indicator);
YacResult yacBindParameter(YacHandle hStmt, YacUint16 id, YacParamDirection direction,YacUint32 extType, YacPointer value, YacInt32 bindSize, YacInt32 bufLength, YacInt32* indicator);
YacResult yacBindParameterByName(YacHandle hStmt, YacChar* name, YacParamDirection direction, YacUint32 extType, YacPointer value, YacInt32 bindSize, YacInt32 bufLength, YacInt32* indicator);

/* YACLI get data APIs */
YacResult yacGetData(YacHandle hStmt, YacUint16 id, YacUint32 rowNumber, YacUint32 extType, YacPointer value, YacInt32 bufLen, YacInt32* indicator);

/* YACLI metadata APIs */
YacResult yacColAttribute(YacHandle hStmt, YacUint16 id, YacColAttr attr, YacVoid* value, YacInt32 bufLen, YacInt32* stringLength);
YacResult yacNumResultCols(YacHandle hStmt, YacInt16* count);
YacResult yacNumParams(YacHandle hStmt, YacUint16* count);

/* YACLI multi result APIs */
YacResult yacStmtGetNextResult(YacHandle hStmt, YacHandle* cursor, YacUint32* rtType);

/* YACLI deprecated lob APIs */
YacResult yacLobDescAlloc(YacHandle hConn, YacType type, YacVoid** desc);
YacResult yacLobDescFree(YacVoid* desc, YacType type);
YacResult yacLobCreateTemporary(YacHandle hConn, YacLobLocator* loc);
YacResult yacLobRead(YacHandle hConn, YacLobLocator* loc, YacUint64* bytes, YacUint8* buf, YacUint64 bufLen);
YacResult yacLobWrite(YacHandle hConn, YacLobLocator* loc, YacUint64* bytes, YacUint8* buf, YacUint64 bufLen);

/* YACLI deprecated lob APIs */
YacResult yacLobDescAlloc(YacHandle hConn, YacType type, YacVoid** desc);
YacResult yacLobDescFree(YacVoid* desc, YacType type);
YacResult yacLobCreateTemporary(YacHandle hConn, YacLobLocator* loc);
YacResult yacLobRead(YacHandle hConn, YacLobLocator* loc, YacUint64* bytes, YacUint8* buf, YacUint64 bufLen);
YacResult yacLobWrite(YacHandle hConn, YacLobLocator* loc, YacUint64* bytes, YacUint8* buf, YacUint64 bufLen);

/* YACLI lob APIs */
YacResult yacLobDescAlloc2(YacHandle hConn, YacLobLocator** desc);
YacResult yacLobDescFree2(YacLobLocator* desc);
YacResult yacLobGetChunkSize(YacHandle hConn, YacLobLocator* locator, YacUint16* chunkSize);
YacResult yacLobGetLength(YacHandle hConn, YacLobLocator* locator, YacUint64* length);
YacResult yacLobFreeTemporary(YacHandle hConn, YacLobLocator* loc);
YacResult yacLobIsTemporary(YacHandle hConn, YacLobLocator* loc, YacBool* isTemporary);
YacResult yacLobTrim(YacHandle hConn, YacLobLocator* loc, YacUint64* newlen);
YacResult yacLobAppend(YacHandle hConn, YacLobLocator* dstLob, YacLobLocator* srcLob);
YacResult yacLobCreateTemporary2(YacHandle hConn, YacLobLocator* loc, YacTempLobType tempLobType);
YacResult yacLobWriteAppend(YacHandle hConn, YacLobLocator* loc, YacUint64* byteSize, YacUint64* charSize, YacUint8* buf, YacUint64 bufLen);
YacResult yacLobWrite2(YacHandle hConn, YacLobLocator* loc, YacUint64* byteSize, YacUint64* charSize, YacUint64 offset, YacUint8* buf, YacUint64 bufLen);
YacResult yacLobRead2(YacHandle hConn, YacLobLocator* loc, YacUint64* byteSize, YacUint64* charSize, YacUint64 offset, YacUint8* buf, YacUint64 bufLen);
YacResult yacLobCharSetId(YacHandle hEnv, const YacLobLocator* loc, YacUint8* csid);
YacResult yacLobIsEqual(YacHandle hEnv, const YacLobLocator* loc1, const YacLobLocator* loc2, YacBool* isEqual);

/* YACLI dataType APIs */
YacResult yacDateGetDate(const YacDate date, YacInt16* year, YacUint8* month, YacUint8* day);
YacResult yacDateSetDate(YacDate* date, YacInt16 year, YacUint8 month, YacUint8 day);

YacResult yacShortTimeGetShortTime(const YacShortTime time, YacUint8* hour, YacUint8* minute, YacUint8* second, YacUint32* fraction);
YacResult yacShortTimeSetShortTime(YacShortTime* time, YacUint8 hour, YacUint8 minute, YacUint8 second, YacUint32 fraction);

YacResult yacTimestampGetTimestamp(const YacTimestamp timestamp, YacInt16* year, YacUint8* month, YacUint8* day, YacUint8* hour, YacUint8* minute, YacUint8* second, YacUint32* fraction);
YacResult yacTimestampSetTimestamp(YacTimestamp* timestamp, YacInt16 year, YacUint8 month, YacUint8 day, YacUint8 hour, YacUint8 minute, YacUint8 second, YacUint32 fraction);

YacResult yacYMIntervalGetYearMonth(const YacYMInterval ymInterval, YacInt32* year, YacInt32* month);
YacResult yacYMIntervalSetYearMonth(YacYMInterval* ymInterval, YacInt32 year, YacInt32 month);

YacResult yacDSIntervalGetDaySecond(const YacDSInterval dsInterval, YacInt32* day, YacInt32* hour, YacInt32* minute, YacInt32* second, YacInt32* fraction);
YacResult yacDSIntervalSetDaySecond(YacDSInterval* dsInterval, YacInt32 day, YacInt32 hour, YacInt32 minute, YacInt32 second, YacInt32 fraction);

YacResult yacDSIntervalFromText(YacHandle hEnv, YacDSInterval* dsInterval, const YacChar* str, YacUint32 strLen);
YacResult yacYMIntervalFromText(YacHandle hEnv, YacYMInterval* ymInterval, const YacChar* str, YacUint32 strLen);

YacResult yacRowIdToText(const YacRowId* rowId, YacChar* str, YacInt32 bufLength, YacInt32* length);
YacResult yacTextToRowId(const YacChar* str, YacInt32 length, YacRowId* rowId);

YacResult yacNumberRound(YacNumber* n, YacInt32 precision, YacInt32 scale);
YacResult yacNumberFromInt(const YacPointer inum, YacUint32 length, YacUint32 flag, YacNumber* number);
YacResult yacNumberToInt(const YacNumber* number, YacUint32 length, YacUint32 flag, YacPointer rsl);
YacResult yacNumberFromText(const YacChar* str, YacUint32 strLength, const YacChar* fmt, YacUint32 fmtLength, const YacChar* nlsParam, YacUint32 nlsParamLength, YacNumber* number);
YacResult yacNumberToText(const YacNumber* number, const YacChar* fmt, YacUint32 fmtLength, const YacChar* nlsParam, YacUint32 nlsParamLength, YacChar* str, YacInt32 bufLength, YacInt32* length);
YacResult yacNumberFromReal(const YacPointer rnum, YacUint32 length, YacNumber* number);
YacResult yacNumberToReal(const YacNumber* number, YacUint32 length, YacPointer rsl);

YacResult yacStringAllocSize(YacHandle hEnv, const YacString* yacString, YacUint32* allocSize);
YacResult yacStringAssign(YacHandle hEnv, const YacString* yacSrcString, YacString** yacDstString);
YacResult yacStringAssignText(YacHandle hEnv, const YacChar* str, YacUint32 strLen, YacString** yacString);
YacChar*  yacStringPtr(YacHandle hEnv, const YacString* yacString);
YacResult yacStringResize(YacHandle hEnv, YacUint32 newSize, YacString** yacString);
YacUint32 yacStringSize(YacHandle hEnv, const YacString* yacString);

YacResult yacRawAllocSize(YacHandle hEnv, const YacRaw* yacRaw, YacUint32* allocSize);
YacResult yacRawAssign(YacHandle hEnv, const YacRaw* yacSrcRaw, YacRaw** yacDstRaw);
YacResult yacRawAssignBytes(YacHandle hEnv, const YacUint8* raw, YacUint32 rawLen, YacRaw** yacRaw);
YacUint8* yacRawPtr(YacHandle hEnv, const YacRaw* yacRaw);
YacResult yacRawResize(YacHandle hEnv, YacUint32 newSize, YacRaw** yacRaw);
YacUint32 yacRawSize(YacHandle hEnv, const YacRaw* yacRaw);

//Yashan External Procedure
#define YEP_RETURN (YacUint16)0xFFFF
typedef YacResult (*YepMethod)(YacHandle hProc);

YacResult yepGetCharsetId(YacHandle hProc, YacUint16* charsetId);
YacResult yepGetBool(YacHandle hProc, YacInt32 id, YacBool* v, YacInt32* lenOrInd);
YacResult yepGetInt8(YacHandle hProc, YacInt32 id, YacInt8* v, YacInt32* lenOrInd);
YacResult yepGetInt16(YacHandle hProc, YacInt32 id, YacInt16* v, YacInt32* lenOrInd);
YacResult yepGetInt32(YacHandle hProc, YacInt32 id, YacInt32* v, YacInt32* lenOrInd);
YacResult yepGetInt64(YacHandle hProc, YacInt32 id, YacInt64* v, YacInt32* lenOrInd);
YacResult yepGetFloat(YacHandle hProc, YacInt32 id, YacFloat* v, YacInt32* lenOrInd);
YacResult yepGetDouble(YacHandle hProc, YacInt32 id, YacDouble* v, YacInt32* lenOrInd);
YacResult yepGetNumber(YacHandle hProc, YacInt32 id, YacNumber* v, YacInt32* lenOrInd);
YacResult yepGetDate(YacHandle hProc, YacInt32 id, YacDate* v, YacInt32* lenOrInd);
YacResult yepGetTimestamp(YacHandle hProc, YacInt32 id, YacTimestamp* v, YacInt32* lenOrInd);
YacResult yepGetYMInterval(YacHandle hProc, YacInt32 id, YacYMInterval* v, YacInt32* lenOrInd);
YacResult yepGetDSInterval(YacHandle hProc, YacInt32 id, YacDSInterval* v, YacInt32* lenOrInd);
YacResult yepGetString(YacHandle hProc, YacInt32 id, YacChar* str, YacUint32 bufSize, YacInt32* lenOrInd);
YacResult yepGetBytes(YacHandle hProc, YacInt32 id, YacUint8* bytes, YacUint32 bufSize, YacInt32* lenOrInd);

YacResult yepOutputNull(YacHandle hProc, YacInt32 id);
YacResult yepOutputBool(YacHandle hProc, YacInt32 id, YacBool v);
YacResult yepOutputInt8(YacHandle hProc, YacInt32 id, YacInt8 v);
YacResult yepOutputInt16(YacHandle hProc, YacInt32 id, YacInt16 v);
YacResult yepOutputInt32(YacHandle hProc, YacInt32 id, YacInt32 v);
YacResult yepOutputInt64(YacHandle hProc, YacInt32 id, YacInt64 v);
YacResult yepOutputFloat(YacHandle hProc, YacInt32 id, YacFloat v);
YacResult yepOutputDouble(YacHandle hProc, YacInt32 id, YacDouble v);
YacResult yepOutputNumber(YacHandle hProc, YacInt32 id, YacNumber* v);
YacResult yepOutputDate(YacHandle hProc, YacInt32 id, YacDate v);
YacResult yepOutputTimestamp(YacHandle hProc, YacInt32 id, YacTimestamp* v);
YacResult yepOutputYMInterval(YacHandle hProc, YacInt32 id, YacYMInterval v);
YacResult yepOutputDSInterval(YacHandle hProc, YacInt32 id, YacDSInterval v);
YacResult yepOutputString(YacHandle hProc, YacInt32 id, YacChar* str);
YacResult yepOutputBytes(YacHandle hProc, YacInt32 id, YacUint8* bytes, YacUint32 size);

#define yepReturnNull(hProc) yepOutputNull(hProc, YEP_RETURN)
#define yepReturnBool(hProc, value) yepOutputBool(hProc, YEP_RETURN, value)
#define yepReturnInt8(hProc, value) yepOutputInt8(hProc, YEP_RETURN, value)
#define yepReturnInt16(hProc, value) yepOutputInt16(hProc, YEP_RETURN, value)
#define yepReturnInt32(hProc, value) yepOutputInt32(hProc, YEP_RETURN, value)
#define yepReturnInt64(hProc, value) yepOutputInt64(hProc, YEP_RETURN, value)
#define yepReturnFloat(hProc, value) yepOutputFloat(hProc, YEP_RETURN, value)
#define yepReturnDouble(hProc, value) yepOutputDouble(hProc, YEP_RETURN, value)
#define yepReturnNumber(hProc, value) yepOutputNumber(hProc, YEP_RETURN, value)
#define yepReturnDate(hProc, value) yepOutputDate(hProc, YEP_RETURN, value)
#define yepReturnTimestamp(hProc, value) yepOutputTimestamp(hProc, YEP_RETURN, value)
#define yepReturnYMInterval(hProc, value) yepOutputYMInterval(hProc, YEP_RETURN, value)
#define yepReturnDSInterval(hProc, value) yepOutputDSInterval(hProc, YEP_RETURN, value)
#define yepReturnString(hProc, value) yepOutputString(hProc, YEP_RETURN, value)
#define yepReturnBytes(hProc, value, size) yepOutputBytes(hProc, YEP_RETURN, value, size)

#ifdef __cplusplus
}
#endif
#endif