//////////////////////////////////////////////////////////////////////////////
/// \file dpxdicom/src/data/dicomdate.cpp
/// \brief Файл с реализацией методов обработки текстовых строк в значениях атрибутов
/// \author Девятников А.В.
/// \date 2025-03-27
///
/// Copyright (C) 2025 by RTK Radiology
/// ALL RIGHTS RESERVED.
//////////////////////////////////////////////////////////////////////////////

#include "dpxdicom/data/dicomdate.h"

#include "dpxcore/utils/stringutils.h"

#include <QDateTime>
#include <QTimeZone>

#include <charconv>
#include <sstream>

static constexpr size_t FMT_YEAR_DIGITS = 4;
static constexpr size_t FMT_MONTH_DIGITS = 2;
static constexpr size_t FMT_DAY_DIGITS = 2;
static constexpr size_t FMT_HOUR_DIGITS = 2;
static constexpr size_t FMT_MINUTE_DIGITS = 2;
static constexpr size_t FMT_SECOND_DIGITS = 2;
static constexpr size_t FMT_FRACTION_DIGITS = 3;
static constexpr size_t FMT_FRACTION_DIGITS_MAX = 6;

static constexpr size_t FMT_SIGN_LEN = 1;
static constexpr size_t FMT_TZ_LENGTH = FMT_SIGN_LEN + FMT_HOUR_DIGITS + FMT_MINUTE_DIGITS;
static constexpr size_t FMT_DATE_LENGTH_MAX = FMT_YEAR_DIGITS + FMT_MONTH_DIGITS + FMT_HOUR_DIGITS;
static constexpr size_t FMT_TIME_LENGTH_MAX
	= FMT_HOUR_DIGITS + FMT_MINUTE_DIGITS + FMT_SECOND_DIGITS + FMT_SIGN_LEN + FMT_FRACTION_DIGITS;
static constexpr size_t FMT_DATE_TIME_LENGTH_MAX = FMT_DATE_LENGTH_MAX + FMT_TIME_LENGTH_MAX + FMT_TZ_LENGTH;

static constexpr std::uint16_t LIM_YEAR_MIN = 1;
static constexpr std::uint16_t LIM_YEAR_MAX = 9999;
static constexpr std::uint16_t LIM_MONTH_MIN = 1;
static constexpr std::uint16_t LIM_MONTH_MAX = 12;
static constexpr std::uint16_t LIM_DAY_MIN = 1;
static constexpr std::uint16_t LIM_DAY_MAX = 31;
static constexpr std::uint16_t LIM_HOUR_MIN = 0;
static constexpr std::uint16_t LIM_HOUR_MAX = 23;
static constexpr std::uint16_t LIM_MINUTE_MIN = 0;
static constexpr std::uint16_t LIM_MINUTE_MAX = 59;
static constexpr std::uint16_t LIM_SECOND_MIN = 0;
static constexpr std::uint16_t LIM_SECOND_MAX = 59;
static constexpr std::uint16_t LIM_FRACTION_MIN = 0;
static constexpr std::uint16_t LIM_FRACTION_MAX = 999;

static constexpr std::uint8_t DAYS_IN_MONTH[12] = {31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31};

/******************************************************************************
 * Реализация функций конвертации для QVariant
 ******************************************************************************/

template<class DicomType> typename DicomType::Native convert_to_native(const DicomType& v) noexcept
{
	return v.toNative();
}

template<class DicomType> DicomType convert_from_native(const typename DicomType::Native& v) noexcept
{
	return DicomType::fromNative(v);
}

template<class DicomType> QByteArray convert_to_bytearray(const DicomType& v) noexcept
{
	QByteArray rv;
	v.toDicom(rv);
	return rv;
}

template<class DicomType> QString convert_to_string(const DicomType& v) noexcept
{
	return QString::fromLatin1(convert_to_bytearray<DicomType>(v));
}

template<class DicomType> DicomType convert_from_bytearray(const QByteArray& v) noexcept
{
	DicomType rv;
	rv.fromDicom(v.constData(), v.constData() + v.size());
	return rv;
}

template<class DicomType> DicomType convert_from_string(const QString& v) noexcept
{
	return convert_from_bytearray<DicomType>(v.toLatin1());
}

AUTORUN()
{
	qRegisterMetaType<DicomTzOffset>();
	QMetaType::registerConverter<DicomTzOffset, QByteArray>(convert_to_bytearray<DicomTzOffset>);
	QMetaType::registerConverter<QByteArray, DicomTzOffset>(convert_from_bytearray<DicomTzOffset>);
	QMetaType::registerConverter<DicomTzOffset, QString>(convert_to_string<DicomTzOffset>);
	QMetaType::registerConverter<QString, DicomTzOffset>(convert_from_string<DicomTzOffset>);

#define DICOMDATE_REG_TYPE(Type)                                                  \
	qRegisterMetaType<Type>();                                                    \
	QMetaType::registerConverter<Type, QByteArray>(convert_to_bytearray<Type>);   \
	QMetaType::registerConverter<QByteArray, Type>(convert_from_bytearray<Type>); \
	QMetaType::registerConverter<Type, QString>(convert_to_string<Type>);         \
	QMetaType::registerConverter<QString, Type>(convert_from_string<Type>);       \
	QMetaType::registerConverter<Type, Type::Native>(convert_to_native<Type>);    \
	QMetaType::registerConverter<Type::Native, Type>(convert_from_native<Type>);  \
	QMetaType::registerDebugStreamOperator<Type>();

	DICOMDATE_REG_TYPE(DicomDate)
	DICOMDATE_REG_TYPE(DicomDateRange)
	DICOMDATE_REG_TYPE(DicomTime)
	DICOMDATE_REG_TYPE(DicomTimeRange)
	DICOMDATE_REG_TYPE(DicomDateTime)
	DICOMDATE_REG_TYPE(DicomDateTimeRange)
	DICOMDATE_REG_TYPE(DicomFindIntRange<quint16>)
	DICOMDATE_REG_TYPE(DicomFindIntRange<qint16>)
	DICOMDATE_REG_TYPE(DicomFindIntRange<quint32>)
	DICOMDATE_REG_TYPE(DicomFindIntRange<qint32>)
	DICOMDATE_REG_TYPE(DicomFindIntRange<quint64>)
	DICOMDATE_REG_TYPE(DicomFindIntRange<qint64>)
	DICOMDATE_REG_TYPE(DicomFindIntRange<double>)
	DICOMDATE_REG_TYPE(DicomFindIntRange<float>)

#undef DICOMDATE_REG_TYPE
}

/******************************************************************************
 * Реализация общих вспосогательных функций
 ******************************************************************************/

/** Проверяет наличие значения в числовом поле \a field.
 * Все поля в #Date и #Time подлежат такой проверке.
 */
template<class T> inline constexpr bool _is_field_set(T field)
{
	return field != std::numeric_limits<T>::max();
};

//-----------------------------------------------------------------------------
/** Возвращает количество дней в месяце \a month с учетом вискосоных годов.
 *
 * Функция предполагает, что используется григорианский календарь. Для дат
 * старее 1582 года используется Пролептический григорианский календарь.
 */
static constexpr std::uint8_t _days_in_month(std::uint16_t year, std::uint8_t month)
{
	if (month == 2 && ((year % 4 == 0 && year % 100 != 0) || year % 400 == 0))
		return 29;
	else if (month >= 1 && month <= 12)
		return DAYS_IN_MONTH[month - 1];
	else
		return 31;
};

/******************************************************************************
 * Реализация вспомогательных функций чтения DICOM текстового значения
 ******************************************************************************/

//-----------------------------------------------------------------------------
/** Разбирает беззнаковое 10-тичное число с фиксированным количеством
 * символов \a fieldLength из входного текста \a p .. \a pstop.
 * \param minValue Минимально разрешенное значение
 * \param maxValue Максимально разрешенное значение
 * \return
 * - \c true, если число успешно разобрано и записано в \a rv, а также указатель
 *   \a p смещен на величину \a fieldLength.
 * - \c false, если произошла одна из ошибок:
 *   - размер предоставленного буфера \a p .. \a pstop меньше,чем \a fieldLength;
 *   - в буфере содержится символ, не являющийся цифрой;
 *   - разобранное число за пределами \a minValue .. \a maxValue
 */
template<class TargetType>
inline bool _parse_field(const char*& p, const char* pstop, size_t fieldLength, std::uint16_t minValue,
						 std::uint16_t maxValue, TargetType& rv)
{
	std::uint16_t tmp {};
	if (p + fieldLength <= pstop && safe_strtoi<10>(p, p + fieldLength, &tmp))
	{
		if (tmp >= minValue && tmp <= maxValue)
		{
			rv = TargetType(tmp);
			p += fieldLength;
			return true;
		}
	}
	return false;
}

//-----------------------------------------------------------------------------
/** Разбирает фракцию секунды в стандартном DICOM формате для "DT" и "TM": ".FFFFFF".
 * Фракция обрезается до миллисекунд и возможные младшие 3 цифры игнорируются.
 * \return
 * - \c true, если фракция успешно разобрана и записана в \a rv, а также указатель
 *   \a p смещен на количество обнаруженных цифр в тексте.
 * - \c false, если буфер начинается не с точки "." и, хотя бы, с одной цифры.
 */
static bool _parse_fraction(const char*& p, const char* pstop, std::uint16_t& rv)
{
	if (p + 2 <= pstop && *p == '.') // 2 = точка и хотя бы одна цифра
	{
		// Определение общего количества подряд идущих цифр. Если их
		// больше 3, то остальные игнорируются.
		const char* pDigitsStop
			= std::find_if(p + FMT_SIGN_LEN, std::min(p + FMT_SIGN_LEN + FMT_FRACTION_DIGITS_MAX, pstop),
						   [](char c) { return c < '0' || c > '9'; });
		std::uint16_t tmp = 0;
		if (pDigitsStop - p > 1
			&& safe_strtoi<10>(p + FMT_SIGN_LEN, std::min(p + FMT_SIGN_LEN + FMT_FRACTION_DIGITS, pDigitsStop), &tmp))
		{
			const size_t digitsCount = size_t(pDigitsStop - p - FMT_SIGN_LEN);
			if (digitsCount == 1)
				tmp *= 100;
			else if (digitsCount == 2)
				tmp *= 10;
			rv = tmp;
			p = pDigitsStop;
			return true;
		}
	}
	return false;
}

//-----------------------------------------------------------------------------
/** Разбирает смещение времени в стандартном DICOM формате "&ZZXX".
 *
 *  Предполагается, что это поле является последним в буфере \a p .. \a pstop
 *  и любой символ после него вызовет ошибку разбора.
 *
 * \return
 * - \c true, если смещение успешно разобрано и записано в \a rv, а также
 *   указатель \a p смещен на 5.
 * - \c false, если произошла одна из ошибок:
 *   - размер предоставленного буфера \a p .. \a pstop не равен \a 5;
 *   - первый символ буфера не '+' и не '-'
 *   - один из символов, кроме первого не является цифрой;
 *   - количество минут больше 59
 *   - смещение за пределами #TzOffset::Min юю #TzOffset::Max
 */
static bool _parse_tz(const char*& p, const char* pstop, DicomTzOffset& rv)
{
	if (p + FMT_TZ_LENGTH != pstop)
		return false;

	std::int32_t sign = 1;
	switch (*p)
	{
	case '-': sign = -1; break;
	case '+': break;
	default:  return false;
	};

	uint16_t hours = 0;
	uint16_t minutes = 0;
	if (!safe_strtoi<10>(p + FMT_SIGN_LEN, p + FMT_SIGN_LEN + FMT_HOUR_DIGITS, &hours)
		|| !safe_strtoi<10>(p + FMT_SIGN_LEN + FMT_HOUR_DIGITS, p + FMT_SIGN_LEN + FMT_HOUR_DIGITS + FMT_MINUTE_DIGITS,
							&minutes))
	{
		return false;
	}
	if (minutes > LIM_MINUTE_MAX)
		return false;
	const std::int32_t seconds = sign * (std::int32_t(hours) * 3600 + std::int32_t(minutes) * 60);
	if (seconds < DicomTzOffset::Min || seconds > DicomTzOffset::Max)
		return false;

	rv.seconds = seconds;
	p += FMT_TZ_LENGTH;

	return true;
}

//-----------------------------------------------------------------------------
/** Разбирает дату в стандартном DICOM формате "YYYYMMDD" с опциональными
 *  компонентами "MM", "DD".
 * \return
 * - \c true, если значение успешно разобрано и записано в \a rv, а также
 *   указатель \a p смещен на количество прочитанных байт.
 * - \c false, если произошла одна из ошибок:
 *   - размер предоставленного буфера \a p .. \a pstop меньше,чем \a 4;
 *   - первые четыре символа не могут быть разобраны как год
 *   - значение года выходит за допустимые пределы
 *   - номер дня выходит за допустимые пределы для года/месяца.
 */
static bool _parse_date(const char*& pbuf, const char* pstop, DicomDate& rv)
{
	DicomDate tmp;
	const char* p = pbuf;
	if (_parse_field(p, pstop, FMT_YEAR_DIGITS, LIM_YEAR_MIN, LIM_YEAR_MAX, tmp.y))
	{
		const bool hasMonthAndDay = _parse_field(p, pstop, FMT_MONTH_DIGITS, LIM_MONTH_MIN, LIM_MONTH_MAX, tmp.m)
								 && _parse_field(p, pstop, FMT_DAY_DIGITS, LIM_DAY_MIN, LIM_DAY_MAX, tmp.d);

		if (!hasMonthAndDay || tmp.d <= _days_in_month(tmp.y, tmp.m))
		{
			pbuf = p;
			rv = tmp;
			return true;
		}
	}

	return false;
}

//-----------------------------------------------------------------------------
/** Разбирает время в стандартном DICOM формате "HHMMSS.FFFFFF" с опциональными
 *  компонентами "MM", "SS" и ".FFFFFF".
 * \return
 * - \c true, если значение успешно разобрано и записано в \a rv, а также
 *   указатель \a p смещен на количество прочитанных байт.
 * - \c false, если произошла одна из ошибок:
 *   - размер предоставленного буфера \a p .. \a pstop меньше,чем \a 2;
 *   - первые два символа не могут быть разобраны как час
 *   - значение часа выходит за допустимые пределы
 */
static bool _parse_time(const char*& p, const char* pstop, DicomTime& rv)
{
	// F.Y.I входные форматы:
	// HH[MM[SS[.F{1,6}]]]
	if (_parse_field(p, pstop, FMT_HOUR_DIGITS, LIM_HOUR_MIN, LIM_HOUR_MAX, rv.h))
	{
		if (_parse_field(p, pstop, FMT_MINUTE_DIGITS, LIM_MINUTE_MIN, LIM_MINUTE_MAX, rv.m))
		{
			if (_parse_field(p, pstop, FMT_SECOND_DIGITS, LIM_SECOND_MIN, LIM_SECOND_MAX, rv.s))
				_parse_fraction(p, pstop, rv.ms);
		}
		return true;
	}
	return false;
}

//-----------------------------------------------------------------------------
/** Разбирает дату и время в стандартном DICOM формате "YYYYMMDDHHMMSS.FFFFFF&ZZXX",
 * где все компоненты опциональны кроме "YYYY"
 * \return
 * - \c true, если значение успешно разобрано и записано в \a rv, а также
 *   указатель \a p смещен на количество прочитанных байт.
 * - \c false, если произошла одна из ошибок:
 *   - размер предоставленного буфера \a p .. \a pstop меньше,чем \a 2;
 *   - первые два символа не могут быть разобраны как год
 *   - значение года выходит за допустимые пределы
 */
static bool _parse_date_time(const char*& p, const char* pstop, DicomDateTime& rv)
{
	// F.Y.I входные форматы:
	// YYYY[MM[DD[HH[MM[SS[.F{1,6}]]]]][+|-ZZXX]

	if (_parse_date(p, pstop, rv.date))
	{
		if (p < pstop)
			_parse_time(p, pstop, rv.time);
		_parse_tz(p, pstop, rv.tzOffset);
		return true;
	}
	return false;
}

/******************************************************************************
 * Реализация вспомогательных функций записи DICOM текстового значения
 ******************************************************************************/

//-----------------------------------------------------------------------------
/** Записывает в \a pd числовое значение \a value длинной \a fieldLenght.
 * Значение предваряется нулями, если необходимо.
 * \return
 * - \c true, если число успешно записано.
 * - \c false, если произошла одна из ошибок:
 *   - в выходном буфере не хватает места для \a fieldLength
 *   - 10-тичных цифр в числе \a value больше, чем \a fieldLength.
 */
static bool _write_field(char*& pd, char* const pdstop, const size_t fieldLength, const std::uint16_t value)
{
	if (pd + fieldLength <= pdstop)
	{
		char charsBuf[std::numeric_limits<std::uint16_t>::digits10];
		const auto result = std::to_chars(charsBuf, charsBuf + sizeof(charsBuf), value);
		if (result.ec == std::errc())
		{
			const size_t charsLength = size_t(result.ptr - charsBuf);
			if (fieldLength >= charsLength)
			{
				if (fieldLength > charsLength)
					memset(pd, '0', fieldLength - charsLength);
				memcpy(pd + fieldLength - charsLength, charsBuf, charsLength);
				pd += fieldLength;
				return true;
			}
		}
	}
	return false;
}

//-----------------------------------------------------------------------------
/** Записывает в \a pd один символ \a sign
 * \return
 * - \c true, если символ успешно записан.
 * - \c false, если в выходном буфере не хватает места.
 */
static bool _write_sign(char*& pd, char* pdstop, char sign)
{
	if (pd + 1 <= pdstop)
	{
		*pd++ = sign;
		return true;
	}
	return false;
}

//-----------------------------------------------------------------------------
/** Записывает в \a pd значение смещения фоосета времени в формате "&ZZXX".
 *
 * Значение \a pd увеличивается на количество записанных символов. Причем, оно
 * может быть увеличено и в случае ошибки.
 *
 * \return
 * - \c true, если оффсет успешно записан (или \a value не валиден и ни одного
 *   байта не записано).
 * - \c false, если в выходном буфере не хватает места.
 */
static bool _write_tz(char*& pd, char* const pdstop, const DicomTzOffset value)
{
	if (!value.isValid())
		return true;

	std::uint16_t h = 0;
	std::uint16_t m = 0;
	char sign = 0;
	if (value.seconds < 0)
	{
		h = std::uint16_t(-value.seconds / 3600);
		m = std::uint16_t((-value.seconds / 60) % 60);
		sign = '-';
	}
	else
	{
		h = std::uint16_t(value.seconds / 3600);
		m = std::uint16_t((value.seconds / 60) % 60);
		sign = '+';
	}

	return _write_sign(pd, pdstop, sign) && _write_field(pd, pdstop, FMT_HOUR_DIGITS, h)
		&& _write_field(pd, pdstop, FMT_MINUTE_DIGITS, m);
}

//-----------------------------------------------------------------------------
/** Записывает в \a pd дату в формате "YYYYMMDD". "пустые" поля заполняются
 * минимально возможными значениями.
 *
 * Значение \a pd увеличивается на количество записанных символов. Причем, оно
 * может быть увеличено и в случае ошибки.
 *
 * \return
 * - \c true, если \a value успешно записано (или \a value пуст и ни одного
 *   байта не записано).
 * - \c false, если в выходном буфере не хватает места.
 */
static bool _write_date_full(char*& pd, char* const pdstop, const DicomDate& value)
{
	if (value.isNull())
		return true;

	return _write_field(pd, pdstop, FMT_YEAR_DIGITS, value.y)
		&& _write_field(pd, pdstop, FMT_MONTH_DIGITS, _is_field_set(value.m) ? value.m : LIM_MONTH_MIN)
		&& _write_field(pd, pdstop, FMT_DAY_DIGITS, _is_field_set(value.d) ? value.d : LIM_DAY_MIN);
}

//-----------------------------------------------------------------------------
/** Записывает в \a pd дату в формате "YYYYMMDD". "пустые" поля не записываются.
 *
 * Значение \a pd увеличивается на количество записанных символов. Причем, оно
 * может быть увеличено и в случае ошибки.
 *
 * \return
 * - \c true, если \a value успешно записано (или \a value пуст и ни одного
 *   байта не записано).
 * - \c false, если в выходном буфере не хватает места.
 */
static bool _write_date_partial(char*& pd, char* const pdstop, const DicomDate& value)
{
	if (!value.isNull())
	{
		if (!_write_field(pd, pdstop, FMT_YEAR_DIGITS, value.y))
			return false;

		if (_is_field_set(value.m))
		{
			if (!_write_field(pd, pdstop, FMT_MONTH_DIGITS, value.m))
				return false;

			if (_is_field_set(value.d) && !_write_field(pd, pdstop, FMT_DAY_DIGITS, value.d))
				return false;
		}
	}

	return true;
}

//-----------------------------------------------------------------------------
/** Записывает в \a pd время в формате "HHMMSS.FFF". "пустые" поля не записываются.
 *
 * Значение \a pd увеличивается на количество записанных символов. Причем, оно
 * может быть увеличено и в случае ошибки.
 *
 * \return
 * - \c true, если \a value успешно записано (или \a value пуст и ни одного
 *   байта не записано).
 * - \c false, если в выходном буфере не хватает места.
 */
static bool _write_time(char*& pd, char* const pdstop, const DicomTime& value)
{
	if (!value.isNull())
	{
		if (!_write_field(pd, pdstop, FMT_HOUR_DIGITS, value.h))
			return false;

		if (_is_field_set(value.m))
		{
			if (!_write_field(pd, pdstop, FMT_MINUTE_DIGITS, value.m))
				return false;

			if (_is_field_set(value.s))
			{
				if (!_write_field(pd, pdstop, FMT_SECOND_DIGITS, value.s))
					return false;

				if (_is_field_set(value.ms))
				{
					if (!_write_sign(pd, pdstop, '.') || !_write_field(pd, pdstop, FMT_FRACTION_DIGITS, value.ms))
						return false;
				}
			}
		}
	}

	return true;
}

//-----------------------------------------------------------------------------
/** Записывает в \a pd дату и время в формате "YYYYMMDDHHMMSS.FFF&ZZXX".
 * "пустые" поля не записываются.
 *
 * Значение \a pd увеличивается на количество записанных символов. Причем, оно
 * может быть увеличено и в случае ошибки.
 *
 * \return
 * - \c true, если \a value успешно записано (или \a value пуст и ни одного
 *   байта не записано).
 * - \c false, если в выходном буфере не хватает места.
 */
static bool _write_date_time(char*& pd, char* const pdstop, const DicomDate& date, const DicomTime& time,
							 const DicomTzOffset tz)
{
	if (date.isNull())
		return true;

	if (time.isNull())
		return _write_date_partial(pd, pdstop, date) && _write_tz(pd, pdstop, tz);

	return _write_date_full(pd, pdstop, date) && _write_time(pd, pdstop, time) && _write_tz(pd, pdstop, tz);
}

//-----------------------------------------------------------------------------
/** Записывает в \a target дату и время \a dt в формате "YYYYMMDDHHMMSS.FFF&ZZXX".
 * "пустые" поля не записываются.
 */
static void _write_date_time(QByteArray& target, const DicomDateTime& dt, bool writeOffset)
{
	char buffer[FMT_DATE_TIME_LENGTH_MAX];
	char* pd = buffer;

	DicomTzOffset tzOutput = writeOffset ? dt.tzOffset : DicomTzOffset();
	if (_write_date_time(pd, buffer + sizeof(buffer), dt.date, dt.time, tzOutput))
	{
		const size_t cbWritten = size_t(pd - buffer);
		target.append(buffer, int(cbWritten));
	}
}

/******************************************************************************
 * TzOffset Implementation
 ******************************************************************************/

//-----------------------------------------------------------------------------
inline DicomTzOffset default_system_tz_offset_at_date(const QDateTime& atDate)
{
	return {QTimeZone::systemTimeZone().offsetFromUtc(atDate)};
}

#ifdef DPX_CONF_ENABLE_TESTS
static auto system_tz_offset_at_date = default_system_tz_offset_at_date;
#endif

//-----------------------------------------------------------------------------
DicomTzOffset DicomTzOffset::system(const QDateTime& atDate) noexcept
{
#ifdef DPX_CONF_ENABLE_TESTS
	return system_tz_offset_at_date(atDate);
#else
	return default_system_tz_offset_at_date(atDate);
#endif
}

//-----------------------------------------------------------------------------
DicomTzOffset DicomTzOffset::system(const DicomDate& atDate) noexcept
{
	QDateTime dateTime = QDateTime(atDate.toNative(), QTime(0, 0, 0), Qt::UTC);
	return system(dateTime);
}

//-----------------------------------------------------------------------------
DicomTzOffset DicomTzOffset::system() noexcept
{
	return system(QDateTime::currentDateTimeUtc());
}

//-----------------------------------------------------------------------------
void DicomTzOffset::toDicom(QByteArray& target) const noexcept
{
	if (isValid())
	{
		char buffer[FMT_TZ_LENGTH];
		char* pd = buffer;
		if (_write_tz(pd, buffer + sizeof(buffer), *this))
		{
			const size_t cbWritten = size_t(pd - buffer);
			Q_ASSERT(cbWritten == sizeof(buffer));
			target.append(buffer, int(cbWritten));
		}
	}
}

//-----------------------------------------------------------------------------
bool DicomTzOffset::fromDicom(const char* p, const char* pstop) noexcept
{
	RTRIM(p, pstop);
	DicomTzOffset rv {Unset};
	if (p != pstop && (!_parse_tz(p, pstop, rv) || p != pstop))
		return false;
	*this = rv;
	return true;
}

/******************************************************************************
 * Date Implementation
 ******************************************************************************/

//-----------------------------------------------------------------------------
DicomDate DicomDate::minimized() const noexcept
{
	return {
		_is_field_set(y) ? y : std::uint16_t(LIM_YEAR_MIN),
		_is_field_set(m) ? m : std::uint8_t(LIM_MONTH_MIN),
		_is_field_set(d) ? d : std::uint8_t(LIM_DAY_MIN),
	};
}

//-----------------------------------------------------------------------------
DicomDate DicomDate::maximized() const noexcept
{
	std::uint16_t year = _is_field_set(y) ? y : std::uint16_t(LIM_YEAR_MAX);
	std::uint8_t month = _is_field_set(m) ? m : std::uint8_t(LIM_MONTH_MAX);
	return {
		year,
		month,
		_is_field_set(d) ? d : _days_in_month(year, month),
	};
}

//-----------------------------------------------------------------------------
QDate DicomDate::toNative() const noexcept
{
	return isNull() ? QDate()
					: QDate(int(y), int(_is_field_set(m) ? m : std::uint8_t(LIM_MONTH_MIN)),
							int(_is_field_set(d) ? d : std::uint8_t(LIM_DAY_MIN)));
}

//-----------------------------------------------------------------------------
DicomDate DicomDate::fromNative(const QDate& value) noexcept
{
	return value.isValid()
			 ? DicomDate {std::uint16_t(value.year()), std::uint8_t(value.month()), std::uint8_t(value.day())}
			 : DicomDate {};
}

//-----------------------------------------------------------------------------
void DicomDate::toDicom(QByteArray& target) const noexcept
{
	if (!isNull())
	{
		char buffer[FMT_DATE_LENGTH_MAX];
		char* pd = buffer;
		if (_write_date_full(pd, buffer + sizeof(buffer), *this))
		{
			const size_t cbWritten = size_t(pd - buffer);
			Q_ASSERT(cbWritten == sizeof(buffer));
			target.append(buffer, int(cbWritten));
		}
	}
}

//-----------------------------------------------------------------------------
bool DicomDate::fromDicom(const char* p, const char* pstop) noexcept
{
	RTRIM(p, pstop);
	DicomDate rv;
	if (p != pstop)
	{
		if (p + FMT_DATE_LENGTH_MAX != pstop || !_parse_date(p, pstop, rv) || p != pstop)
			return false;
	}
	*this = rv;
	return true;
}

/******************************************************************************
 * DateRange Implementation
 ******************************************************************************/

//-----------------------------------------------------------------------------
QPair<QDate, QDate> DicomDateRange::toNative() const noexcept
{
	return {
		!from.isNull() ? from.minimized().toNative() : QDate(),
		!to.isNull() ? to.maximized().toNative() : QDate(),
	};
}

//-----------------------------------------------------------------------------
DicomDateRange DicomDateRange::fromNative(const QPair<QDate, QDate>& value) noexcept
{
	return {
		DicomDate::fromNative(value.first),
		DicomDate::fromNative(value.second),
	};
}

//-----------------------------------------------------------------------------
void DicomDateRange::toDicom(QByteArray& target) const noexcept
{
	if (!isNull())
	{
		from.toDicom(target);
		target += '-';
		to.toDicom(target);
	}
}

//-----------------------------------------------------------------------------
bool DicomDateRange::fromDicom(const char* p, const char* pstop) noexcept
{
	// F.Y.I входные форматы:
	// [Date][-[Date]]

	RTRIM(p, pstop);

	DicomDateRange tmp;

	if (p != pstop)
	{
		const char* pDelimtier = safe_strchr(p, pstop, '-');
		if (pDelimtier)
		{
			if (p != pDelimtier && (!_parse_date(p, pDelimtier, tmp.from) || p != pDelimtier))
				return false;
			p = pDelimtier + 1;
			if (p != pstop && (!_parse_date(p, pstop, tmp.to) || p != pstop))
				return false;
			if (tmp.from.isNull() && tmp.to.isNull())
				return false;
		}
		else
		{
			if (!_parse_date(p, pstop, tmp.from) || p != pstop)
				return false;
			tmp.to = tmp.from;
		}
	}

	*this = tmp;
	return true;
}

/******************************************************************************
 * Time Implementation
 ******************************************************************************/

//-----------------------------------------------------------------------------
DicomTime DicomTime::minimized() const noexcept
{
	return {
		_is_field_set(h) ? h : std::uint8_t(LIM_HOUR_MIN),
		_is_field_set(m) ? m : std::uint8_t(LIM_MINUTE_MIN),
		_is_field_set(s) ? s : std::uint8_t(LIM_SECOND_MIN),
		_is_field_set(ms) ? ms : std::uint16_t(LIM_FRACTION_MIN),
	};
}

//-----------------------------------------------------------------------------
DicomTime DicomTime::maximized() const noexcept
{
	return {
		_is_field_set(h) ? h : std::uint8_t(LIM_HOUR_MAX),
		_is_field_set(m) ? m : std::uint8_t(LIM_MINUTE_MAX),
		_is_field_set(s) ? s : std::uint8_t(LIM_SECOND_MAX),
		_is_field_set(ms) ? ms : std::uint16_t(LIM_FRACTION_MAX),
	};
}

//-----------------------------------------------------------------------------
QTime DicomTime::toNative() const noexcept
{
	return isNull() ? QTime()
					: QTime(int(h), int(_is_field_set(m) ? m : LIM_MINUTE_MIN),
							int(_is_field_set(s) ? s : LIM_SECOND_MIN), int(_is_field_set(ms) ? ms : LIM_FRACTION_MIN));
}

//-----------------------------------------------------------------------------
DicomTime DicomTime::fromNative(const QTime& value) noexcept
{
	DicomTime rv;
	if (value.isValid())
	{
		rv.h = value.hour();
		rv.m = value.minute();
		rv.s = value.second();
		if (value.msec())
			rv.ms = value.msec();
	}
	return rv;
}

//-----------------------------------------------------------------------------
void DicomTime::toDicom(QByteArray& target) const noexcept
{
	if (!isNull())
	{
		char buffer[FMT_TIME_LENGTH_MAX];
		char* pd = buffer;
		if (_write_time(pd, buffer + sizeof(buffer), *this))
		{
			const size_t cbWritten = size_t(pd - buffer);
			target.append(buffer, int(cbWritten));
		}
	}
}

//-----------------------------------------------------------------------------
bool DicomTime::fromDicom(const char* p, const char* pstop) noexcept
{
	RTRIM(p, pstop);
	DicomTime rv;
	if (p != pstop && (!_parse_time(p, pstop, rv) || p != pstop))
		return false;
	*this = rv;
	return true;
}

/******************************************************************************
 * TimeRange Implementation
 ******************************************************************************/

//-----------------------------------------------------------------------------
QPair<QTime, QTime> DicomTimeRange::toNative() const noexcept
{
	return {
		!from.isNull() ? from.minimized().toNative() : QTime(),
		!to.isNull() ? to.maximized().toNative() : QTime(),
	};
}

//-----------------------------------------------------------------------------
DicomTimeRange DicomTimeRange::fromNative(const QPair<QTime, QTime>& value) noexcept
{
	return {
		DicomTime::fromNative(value.first),
		DicomTime::fromNative(value.second),
	};
}

//-----------------------------------------------------------------------------
void DicomTimeRange::toDicom(QByteArray& target) const noexcept
{
	if (!isNull())
	{
		from.toDicom(target);
		target += '-';
		to.toDicom(target);
	}
}

//-----------------------------------------------------------------------------
bool DicomTimeRange::fromDicom(const char* p, const char* pstop) noexcept
{
	// F.Y.I входные форматы:
	// [Time][-[Time]]

	RTRIM(p, pstop);

	DicomTimeRange tmp;
	if (p != pstop)
	{
		const char* pDelimtier = safe_strchr(p, pstop, '-');
		if (pDelimtier)
		{
			if (!tmp.from.fromDicom(p, pDelimtier))
				return false;
			if (!tmp.to.fromDicom(pDelimtier + 1, pstop))
				return false;
			if (tmp.from.isNull() && tmp.to.isNull())
				return false;
		}
		else
		{
			if (!_parse_time(p, pstop, tmp.from) || p != pstop)
				return false;
			tmp.to = tmp.from;
		}
	}

	*this = tmp;
	return true;
}

/******************************************************************************
 * DateTime Implementation
 ******************************************************************************/

//-----------------------------------------------------------------------------
DicomDateTime DicomDateTime::minimized() const noexcept
{
	return {
		date.minimized(),
		time.minimized(),
		tzOffset,
	};
}

//-----------------------------------------------------------------------------
DicomDateTime DicomDateTime::maximized() const noexcept
{
	return {
		date.maximized(),
		time.maximized(),
		tzOffset,
	};
}

//-----------------------------------------------------------------------------
QDateTime DicomDateTime::toNative() const noexcept
{
	QDateTime rv;
	if (!isNull())
	{
		if (tzOffset.seconds == 0)
		{
			rv = QDateTime(date.toNative(), time.toNative(), Qt::UTC);
		}
		else if (tzOffset.isValid())
		{
			rv = QDateTime(date.toNative(), time.toNative(), Qt::OffsetFromUTC, tzOffset.seconds);
		}
		else
		{
			rv = QDateTime(date.toNative(), time.toNative());
		}
	}
	return rv;
}

//-----------------------------------------------------------------------------
DicomDateTime DicomDateTime::fromNative(const QDateTime& value) noexcept
{
	DicomDateTime rv;
	if (value.isValid())
	{
		rv.date = DicomDate::fromNative(value.date());
		rv.time = DicomTime::fromNative(value.time());
		if (value.timeSpec() != Qt::LocalTime)
			rv.tzOffset = DicomTzOffset {value.offsetFromUtc()};
	}
	return rv;
}

//-----------------------------------------------------------------------------
void DicomDateTime::toDicom(QByteArray& target, bool isCFindRq, bool alwaysWriteOffset,
							DicomTzOffset offsetInDataset) const noexcept
{
	if (!isNull())
	{
		bool writeOffset = false;
		bool becomesMoreSpecific = false;
		DicomDateTime adjusted = adjustToNewDatasetOffset(isCFindRq, alwaysWriteOffset, offsetInDataset,
														  &becomesMoreSpecific, &writeOffset);

		if (isCFindRq && becomesMoreSpecific)
		{
			adjusted = minimized().adjustToNewDatasetOffset(isCFindRq, alwaysWriteOffset, offsetInDataset, nullptr,
															&writeOffset);
			_write_date_time(target, adjusted, writeOffset);
			target += '-';
			adjusted = maximized().adjustToNewDatasetOffset(isCFindRq, alwaysWriteOffset, offsetInDataset, nullptr,
															&writeOffset);
			_write_date_time(target, adjusted, writeOffset);
		}
		else
		{
			_write_date_time(target, adjusted, writeOffset);
		}
	}
}

//-----------------------------------------------------------------------------
bool DicomDateTime::fromDicom(const char* p, const char* pstop, bool isCFindRq, DicomTzOffset offsetInDataset) noexcept
{
	RTRIM(p, pstop);
	DicomDateTime rv;
	if (p != pstop)
	{
		if (!_parse_date_time(p, pstop, rv) || p != pstop)
			return false;
		if (isCFindRq && rv.tzOffset.isNegative())
			return false;
		if (rv.tzOffset.isNull())
		{
			rv.tzOffset = offsetInDataset;
			rv.tzSetFromDataset = true;
		}
	}
	*this = rv;
	return true;
}

//-----------------------------------------------------------------------------
DicomDateTime DicomDateTime::adjustToNewDatasetOffset(bool isCFindRq, bool alwaysWriteOffset,
													  DicomTzOffset offsetInDataset, bool* rvBecomesMoreSpecific,
													  bool* rvOffsetWriteRequired) const noexcept
{
	DicomDateTime rv = *this;
	bool becomesMoreSpecific = false;
	bool writeOffset = alwaysWriteOffset; // Признак необходимости записи tzFinal в буфер

	if (!isNull())
	{
		// В случае "отрицательных" оффсетов для C-FIND, необходимо пеобразовать
		// дату в UTC
		//
		// DICOM PS3.4  C.2.2.2 Attribute Matching:
		//
		// Note:
		// 1. For example, the "-" character is not valid for the DA, DT and TM
		// VRs but is used for Range Matching.
		//
		// DICOM PS3.4 C.2.2.2.1 Single Value Matching:
		//
		// b. of VR of DA, TM or DT and contains a single value with no "-" and
		// no QUOTATION MARK characters, or
		//
		// DICOM PS3.4 C.2.2.2.1.3 Attributes of VR of DA, DT or TM
		//
		// Note:
		// 3. Exclusion of the "-" character for Single Value Matching implies
		// that a Key Attribute with a VR of DT may not contain a negative offset
		// from Universal Coordinated Time (UTC) if Single Value Matching is
		// intended. Use of the "-" character in values of VR of TM, DA and DT
		// indicates Range Matching.
		// 4. If an application is in a local time zone that has a negative offset
		// then it cannot perform Single Value Matching using a local time notation.
		// Instead, it can convert the Key Attribute Value to UTC and use an
		// explicit suffix of "+0000".
		//

		// Количество секунд на которое нужно сместить текущее время для того, чтобы оно оказалось
		// во временное зоне tzFinal
		int tzAdjustmentSec = 0;

		if (rv.tzOffset.isNull())
		{
			if (offsetInDataset.isNull() || !tzSetFromDataset)
			{
				if (alwaysWriteOffset)
					rv.tzOffset = DicomTzOffset::system(date);
			}
			else
			{
				DicomTzOffset current = DicomTzOffset::system(date);
				tzAdjustmentSec = offsetInDataset.seconds - current.seconds;
				rv.tzOffset = offsetInDataset;
			}
		}
		else if (tzSetFromDataset)
		{
			if (offsetInDataset.isNull())
			{
				DicomTzOffset current = DicomTzOffset::system(date);
				tzAdjustmentSec = current.seconds - rv.tzOffset.seconds;
				rv.tzOffset = current;
			}
			else if (!(offsetInDataset == rv.tzOffset))
			{
				tzAdjustmentSec = offsetInDataset.seconds - rv.tzOffset.seconds;
				rv.tzOffset = offsetInDataset;
			}
		}
		else
		{
			writeOffset = true;
		}


		if (isCFindRq && rv.tzOffset.isNegative())
		{
			tzAdjustmentSec -= rv.tzOffset.seconds;
			rv.tzOffset.seconds = 0;
		}

		if (tzAdjustmentSec)
		{
			quint64 timestamp = toNative().toMSecsSinceEpoch() + tzAdjustmentSec * 1000i64;
			QDateTime native = QDateTime::fromMSecsSinceEpoch(timestamp, Qt::OffsetFromUTC, rv.tzOffset.seconds);
			rv.date = DicomDate::fromNative(native.date());
			rv.time = DicomTime::fromNative(native.time());

			// Очищаем поля из выхода, если их нет в текущем объекте и смена смещения во времени
			// не создала необходимости в их выводе.
			if (!_is_field_set(time.ms))
			{
				rv.time.ms = time.ms;
				if (!_is_field_set(time.s))
				{
					if ((tzAdjustmentSec % 60) != 0)
					{
						becomesMoreSpecific = true;
					}
					else
					{
						rv.time.s = time.s;
						if (!_is_field_set(time.m))
						{
							if ((tzAdjustmentSec % 3600) != 0)
							{
								becomesMoreSpecific = true;
							}
							else
							{
								rv.time.m = time.m;
								if (!_is_field_set(time.h))
									becomesMoreSpecific = true;
							}
						}
					}
				}
			}
		}

		if (alwaysWriteOffset)
			rv.tzSetFromDataset = false;
	}

	if (rvBecomesMoreSpecific)
		*rvBecomesMoreSpecific = becomesMoreSpecific;

	if (rvOffsetWriteRequired)
		*rvOffsetWriteRequired = writeOffset;

	return rv;
}

/******************************************************************************
 * DateTimeRange Implementation
 ******************************************************************************/

//-----------------------------------------------------------------------------
QPair<QDateTime, QDateTime> DicomDateTimeRange::toNative() const noexcept
{
	return {
		!from.isNull() ? from.minimized().toNative() : QDateTime(),
		!to.isNull() ? to.maximized().toNative() : QDateTime(),
	};
}

//-----------------------------------------------------------------------------
DicomDateTimeRange DicomDateTimeRange::fromNative(const QPair<QDateTime, QDateTime>& value) noexcept
{
	return {
		DicomDateTime::fromNative(value.first),
		DicomDateTime::fromNative(value.second),
	};
}

//-----------------------------------------------------------------------------
void DicomDateTimeRange::toDicom(QByteArray& target, bool alwaysWriteOffset, DicomTzOffset offsetInDataset) const noexcept
{
	if (!isNull())
	{
		if (from == to)
		{
			from.toDicom(target, true, alwaysWriteOffset, offsetInDataset);
		}
		else
		{
			if (!from.isNull())
			{
				bool writeOffset = false;
				auto adjusted = from.adjustToNewDatasetOffset(true, alwaysWriteOffset, offsetInDataset, nullptr,
																&writeOffset);
				_write_date_time(target, adjusted, writeOffset);
			}
			target += '-';
			if (!to.isNull())
			{
				bool writeOffset = false;
				auto adjusted = to.adjustToNewDatasetOffset(true, alwaysWriteOffset, offsetInDataset, nullptr,
															  &writeOffset);
				_write_date_time(target, adjusted, writeOffset);
			}
		}
	}
}

//-----------------------------------------------------------------------------
bool DicomDateTimeRange::fromDicom(const char* p, const char* pstop, DicomTzOffset offsetInDataset) noexcept
{
	// F.Y.I входные форматы:
	// [Time][-[Time]]

	RTRIM(p, pstop);

	DicomDateTimeRange tmp;
	if (p != pstop)
	{
		const char* pDelimiter = safe_strchr(p, pstop, '-');
		if (pDelimiter)
		{
			if (!tmp.from.fromDicom(p, pDelimiter, true, offsetInDataset))
				return false;
			if (!tmp.to.fromDicom(pDelimiter + 1, pstop, true, offsetInDataset))
				return false;
			if (tmp.from.isNull() && tmp.to.isNull())
				return false;
		}
		else
		{
			if (!tmp.from.fromDicom(p, pstop, true, offsetInDataset))
				return false;
			tmp.to = tmp.from;
		}
	}

	*this = tmp;
	return true;
}

/******************************************************************************
 * Операторы отладочного вывода в QDebug
 ******************************************************************************/

template<class T> inline QDebug implement_operator_QDebug(QDebug debug, const char* name, const T& value)
{
	QDebugStateSaver saver(debug);
	debug.nospace().noquote() << name << '(';
	if (!value.isNull())
	{
		QByteArray s;
		value.toDicom(s);
		debug << s;
	}
	debug << ')';
	return debug;
}

//-----------------------------------------------------------------------------
QDebug operator<< (QDebug debug, const DicomTzOffset& value)
{
	return implement_operator_QDebug(debug, "DicomTzOffset", value);
}

//-----------------------------------------------------------------------------
QDebug operator<< (QDebug debug, const DicomDate& value)
{
	return implement_operator_QDebug(debug, "DicomDate", value);
}

//-----------------------------------------------------------------------------
QDebug operator<< (QDebug debug, const DicomDateRange& value)
{
	return implement_operator_QDebug(debug, "DicomDateRange", value);
}

//-----------------------------------------------------------------------------
QDebug operator<< (QDebug debug, const DicomTime& value)
{
	return implement_operator_QDebug(debug, "DicomTime", value);
}

//-----------------------------------------------------------------------------
QDebug operator<< (QDebug debug, const DicomTimeRange& value)
{
	return implement_operator_QDebug(debug, "DicomTimeRange", value);
}

//-----------------------------------------------------------------------------
QDebug operator<< (QDebug debug, const DicomDateTime& value)
{
	return implement_operator_QDebug(debug, "DicomDateTime", value);
}

//-----------------------------------------------------------------------------
QDebug operator<< (QDebug debug, const DicomDateTimeRange& value)
{
	return implement_operator_QDebug(debug, "DicomDateTimeRange", value);
}


#ifdef DPX_CONF_ENABLE_TESTS

#	define DOCTEST_CONFIG_REQUIRE_STRINGIFICATION_FOR_ALL_USED_TYPES

#	include "dpxcore/test/catch.hpp"

extern const int test_lib_dpxdicom_dicomdate_cpp = 0;

/* clazy:excludeall=non-pod-global-static */

/******************************************************************************
 * Операторы отладочного вывода в std::ostream
 ******************************************************************************/

template<class T>
inline std::ostream& implement_operator_std_ostream(std::ostream& os, const char* name, const T& value)
{
	os << name << '(';
	if (!value.isNull())
	{
		QByteArray s;
		value.toDicom(s);
		os << s.constData();
	}
	os << ')';
	return os;
}

//-----------------------------------------------------------------------------
static std::ostream& operator<< (std::ostream& os, const DicomTzOffset& value)
{
	return implement_operator_std_ostream(os, "DicomTzOffset", value);
}

//-----------------------------------------------------------------------------
static std::ostream& operator<< (std::ostream& os, const DicomDate& value)
{
	return implement_operator_std_ostream(os, "DicomDate", value);
}

//-----------------------------------------------------------------------------
static std::ostream& operator<< (std::ostream& os, const DicomDateRange& value)
{
	return implement_operator_std_ostream(os, "DicomDateRange", value);
}

//-----------------------------------------------------------------------------
static std::ostream& operator<< (std::ostream& os, const DicomTime& value)
{
	return implement_operator_std_ostream(os, "DicomTime", value);
}

//-----------------------------------------------------------------------------
static std::ostream& operator<< (std::ostream& os, const DicomTimeRange& value)
{
	return implement_operator_std_ostream(os, "DicomTimeRange", value);
}

//-----------------------------------------------------------------------------
static std::ostream& operator<< (std::ostream& os, const DicomDateTime& value)
{
	return implement_operator_std_ostream(os, "DicomDateTime", value);
}

//-----------------------------------------------------------------------------
static std::ostream& operator<< (std::ostream& os, const DicomDateTimeRange& value)
{
	return implement_operator_std_ostream(os, "DicomDateTimeRange", value);
}

//-----------------------------------------------------------------------------
std::ostream& operator<< (std::ostream& os, const QDate& value)
{
	if (value.isNull())
		os << "QDate()";
	else
		os << "QDate(" << value.toString(Qt::ISODate).toStdString() << ")";
	return os;
}

//-----------------------------------------------------------------------------
std::ostream& operator<< (std::ostream& os, const QPair<QDate, QDate>& value)
{
	if (value.first.isNull() && value.second.isNull())
	{
		os << "QPair<QDate,QDate>()";
	}
	else
	{
		os << "QPair<QDate,QDate>(" << value.first.toString(Qt::ISODate).toStdString() << ", "
		   << value.second.toString(Qt::ISODate).toStdString() << ")";
	}
	return os;
}

//-----------------------------------------------------------------------------
std::ostream& operator<< (std::ostream& os, const QTime& value)
{
	if (value.isNull())
		os << "QTime()";
	else
		os << "QTime(" << value.toString(Qt::ISODateWithMs).toStdString() << ")";
	return os;
}

//-----------------------------------------------------------------------------
std::ostream& operator<< (std::ostream& os, const QPair<QTime, QTime>& value)
{
	if (value.first.isNull() && value.second.isNull())
	{
		os << "QPair<QTime,QTime>()";
	}
	else
	{
		os << "QPair<QTime,QTime>(" << value.first.toString(Qt::ISODateWithMs).toStdString() << ", "
		   << value.second.toString(Qt::ISODateWithMs).toStdString() << ")";
	}
	return os;
}

//-----------------------------------------------------------------------------
std::ostream& operator<< (std::ostream& os, const QDateTime& value)
{
	if (value.isNull())
		os << "QDateTime()";
	else
		os << "QDateTime(" << value.toString(Qt::ISODateWithMs).toStdString() << ")";
	return os;
}

//-----------------------------------------------------------------------------
std::ostream& operator<< (std::ostream& os, const QPair<QDateTime, QDateTime>& value)
{
	if (value.first.isNull() && value.second.isNull())
	{
		os << "QPair<QDateTime,QDateTime>()";
	}
	else
	{
		os << "QPair<QDateTime,QDateTime>(" << value.first.toString(Qt::ISODateWithMs).toStdString() << ", "
		   << value.second.toString(Qt::ISODateWithMs).toStdString() << ")";
	}
	return os;
}

namespace tests {

TEST_CASE("[dpxdicom.dicomdate] DicomTzOffset can parse dicom")
{
	struct TestData // NOLINT
	{
		const char* input;
		bool parsedOk;
		DicomTzOffset value;
		bool isNull;
		bool isValid;
		bool isNegative;
	};

#	define TZP(h, m) DicomTzOffset {h * 3600 + m * 60}
#	define TZM(h, m)            \
		DicomTzOffset            \
		{                        \
			-(h * 3600 + m * 60) \
		}
	static const TestData TEST_DATA[] = {
		{"", true, {}, true, false, false},
		{"+0000", true, {0}, false, true, false},
		{"-0000", true, {0}, false, true, false},
		{"+0001", true, TZP(0, 1), false, true, false},
		{"-0001", true, TZM(0, 1), false, true, true},
		{"+0100", true, TZP(1, 0), false, true, false},
		{"-0100", true, TZM(1, 0), false, true, true},
		{"+0059", true, TZP(0, 59), false, true, false},
		{"-0059", true, TZM(0, 59), false, true, true},
		{"+0060", false, {}, true, false, false},
		{"-0060", false, {}, true, false, false},
		{"+1400", true, TZP(14, 0), false, true, false},
		{"-1200", true, TZM(12, 0), false, true, true},
		{"+1500", false, {}, true, false, false},
		{"-1300", false, {}, true, false, false},
		{"+1401", false, {}, true, false, false},
		{"-1201", false, {}, true, false, false},
		{"+1359", true, TZP(13, 59), false, true, false},
		{"-1159", true, TZM(11, 59), false, true, true},
		{"0000", false, {}, true, false, false},
		{"z0000", false, {}, true, false, false},
		{"-", false, {}, true, false, false},
		{"+", false, {}, true, false, false},
		{"+0", false, {}, true, false, false},
		{"+00", false, {}, true, false, false},
		{"+000", false, {}, true, false, false},
		{"+00000", false, {}, true, false, false},
		{"+0000a", false, {}, true, false, false},
		{"+000a", false, {}, true, false, false},
		{"+00a", false, {}, true, false, false},
		{"+0a", false, {}, true, false, false},
		{"+a", false, {}, true, false, false},
	};

	for (const auto& test : TEST_DATA)
	{
		CAPTURE(test.input);
		DicomTzOffset parsed;
		bool parsedOk = parsed.fromDicom(test.input, test.input + strlen(test.input));
		CHECK_EQ(parsedOk, test.parsedOk);
		CHECK_EQ(parsed, test.value);
		CHECK_EQ(parsed.isNull(), test.isNull);
		CHECK_EQ(parsed.isValid(), test.isValid);
		CHECK_EQ(parsed.isNegative(), test.isNegative);
	}
}

TEST_CASE("[dpxdicom.cdataset_parsers] TzOffset can write dicom")
{
	struct TestData // NOLINT
	{
		DicomTzOffset input;
		const char* expected;
	};

	static const TestData TEST_DATA[] = {
		{{}, ""},
		{TZP(0, 0), "+0000"},
		{TZP(0, 1), "+0001"},
		{TZM(0, 1), "-0001"},
		{TZP(1, 0), "+0100"},
		{TZM(1, 0), "-0100"},
		{TZP(0, 59), "+0059"},
		{TZM(0, 59), "-0059"},
		{TZP(14, 00), "+1400"},
		{TZM(12, 00), "-1200"},
		{TZP(14, 01), ""},
		{TZM(12, 01), ""},
	};

	for (const auto& test : TEST_DATA)
	{
		CAPTURE(test.input);
		QByteArray written;
		test.input.toDicom(written);
		CHECK_EQ(written, test.expected);
	}
}

TEST_CASE("[dpxdicom.dicomdate] Date can parse dicom")
{
	struct TestData // NOLINT
	{
		const char* input;
		bool parsedOk;
		DicomDate value = {};
		bool isNull = true;
	};

	static const TestData TEST_DATA[] = {
		{"", true},
		{"20010203", true, {2001, 02, 03}, false},
		{"00010101", true, {1, 1, 1}, false},
		{"00010101 ", true, {1, 1, 1}, false},
		{" 00010101", false, {}},
		{"99991231", true, {9999, 12, 31}, false},
		{"00000101", false, {}},
		{"00010001", false, {}},
		{"00010100", false, {}},
		{"00011301", false, {}},
		{"00010132", false, {}},
		{"20240229", true, {2024, 02, 29}, false},
		{"20230229", false, {}},
		{"0001010", false, {}},
		{"000101", false, {}},
		{"00010", false, {}},
		{"0001", false, {}},
		{"000", false, {}},
		{"00", false, {}},
		{"0", false, {}},
		{"a", false, {}},
		{"000z0101", false, {}},
		{"00010z01", false, {}},
		{"0001010z", false, {}},
		{"00010101z", false, {}},
		{"000101+1", false},
		{"000101-1", false},
		{"0001+101", false},
		{"0001-101", false},
		{"+0010101", false},
		{"-0010101", false},
	};

	for (const auto& test : TEST_DATA)
	{
		CAPTURE(test.input);
		DicomDate parsed;
		bool parsedOk = parsed.fromDicom(test.input, test.input + strlen(test.input));
		CHECK_EQ(parsedOk, test.parsedOk);
		CHECK_EQ(parsed, test.value);
		CHECK_EQ(parsed.isNull(), test.isNull);
	}
}

TEST_CASE("[dpxdicom.dicomdate] Date can write dicom")
{
	struct TestData // NOLINT
	{
		DicomDate input;
		const char* expected;
	};

	static const TestData TEST_DATA[] = {
		{{}, ""},
		{{2001}, "20010101"},
		{{2001, 2}, "20010201"},
		{{2001, 2, 3}, "20010203"},
		{{0, 0, 0}, "00000000"},
		{{9999, 99, 99}, "99999999"},
		{{10000, 99, 99}, ""},
		{{9999, 100, 99}, ""},
		{{9999, 99, 100}, ""},
	};

	for (const auto& test : TEST_DATA)
	{
		CAPTURE(test.input);
		QByteArray written;
		test.input.toDicom(written);
		CHECK_EQ(written, test.expected);
	}
}

TEST_CASE("[dpxdicom.dicomdate] Date can convert native")
{
	struct TestData // NOLINT
	{
		DicomDate own;
		QDate native;
	};

	static const TestData TEST_DATA[] = {
		{{}, {}},
		{{1, 2, 3}, {1, 2, 3}},
		{{9999, 2, 3}, {9999, 2, 3}},
	};

	static const auto fmt = [](const TestData& d)
	{
		std::ostringstream ss;
		ss << "TestData(" << d.own << ", " << d.native << ")";
		return ss.str();
	};

	for (const auto& test : TEST_DATA)
	{
		CAPTURE(fmt(test));
		auto own2native = test.own.toNative();
		CHECK_EQ(own2native, test.native);
		auto native2own = DicomDate::fromNative(test.native);
		CHECK_EQ(native2own, test.own);
	}
}

TEST_CASE("[dpxdicom.dicomdate] DateRange can parse dicom")
{
	struct TestData // NOLINT
	{
		const char* input;
		bool parsedOk;
		DicomDateRange value = {};
		bool isNull = true;
	};

	static const TestData TEST_DATA[] = {
		{"", true},
		{"20010203", true, {{2001, 2, 3}, {2001, 2, 3}}, false},
		{"20010203 ", true, {{2001, 2, 3}, {2001, 2, 3}}, false},
		{"-20010203", true, {{}, {2001, 2, 3}}, false},
		{"20010203-", true, {{2001, 2, 3}, {}}, false},
		{"200102-", true, {{2001, 2}, {}}, false},
		{"2001-", true, {{2001}, {}}, false},
		{"2001- ", true, {{2001}, {}}, false},
		{"-", false},
		{"-2001", true, {{}, {2001}}, false},
		{"-200102", true, {{}, {2001, 2}}, false},
		{"-20010203", true, {{}, {2001, 2, 3}}, false},
		{"-20010203 ", true, {{}, {2001, 2, 3}}, false},
		{"200z- ", false},
		{"-200z", false},
		{"-2001z", false},
		{"2001-2002", true, {{2001}, {2002}}, false},
		{"20010203-20040506", true, {{2001, 2, 3}, {2004, 5, 6}}, false},
		{"20010203-20040506z", false},
		{" 20010203-20040506", false},
	};

	for (const auto& test : TEST_DATA)
	{
		CAPTURE(test.input);
		DicomDateRange parsed;
		bool parsedOk = parsed.fromDicom(test.input, test.input + strlen(test.input));
		CHECK_EQ(parsedOk, test.parsedOk);
		CHECK_EQ(parsed, test.value);
		CHECK_EQ(parsed.isNull(), test.isNull);
	}
}

TEST_CASE("[dpxdicom.dicomdate] DateRange can write dicom")
{
	struct TestData // NOLINT
	{
		DicomDateRange input;
		const char* expected;
	};

	static const TestData TEST_DATA[] = {
		{{}, ""},
		{{{2001, 2, 3}, {2004, 5, 6}}, "20010203-20040506"},
		{{{2001}, {2001}}, "20010101-20010101"},
		{{{2001}, {2021}}, "20010101-20210101"},
		{{{2001}, {}}, "20010101-"},
		{{{}, {2001}}, "-20010101"},
	};

	for (const auto& test : TEST_DATA)
	{
		CAPTURE(test.input);
		QByteArray written;
		test.input.toDicom(written);
		CHECK_EQ(written, test.expected);
	}
}

TEST_CASE("[dpxdicom.dicomdate] DateRange can convert native")
{
	struct TestData // NOLINT
	{
		DicomDateRange own;
		QPair<QDate, QDate> native;
		bool native2own = true;
		bool own2native = true;
	};

	static const TestData TEST_DATA[] = {
		{{}, {}, true, true},
		{{{1, 2, 3}, {}}, {{1, 2, 3}, {}}},
		{{{}, {1, 2, 3}}, {{}, {1, 2, 3}}},
		{{{1, 2, 3}, {4, 5, 6}}, {{1, 2, 3}, {4, 5, 6}}},
		{{{1, 2, 3}, {1, 2, 3}}, {{1, 2, 3}, {1, 2, 3}}},
		{{{1}, {2}}, {{1, 1, 1}, {2, 12, 31}}, false},
		{{{1, 2}, {2, 4}}, {{1, 2, 1}, {2, 4, 30}}, false},
	};

	static const auto fmt = [](const TestData& d)
	{
		std::ostringstream ss;
		ss << "TestData(" << d.own << ", " << d.native << ")";
		return ss.str();
	};

	for (const auto& test : TEST_DATA)
	{
		CAPTURE(fmt(test));
		if (test.own2native)
		{
			auto own2native = test.own.toNative();
			CHECK_EQ(own2native, test.native);
		}
		if (test.native2own)
		{
			auto native2own = DicomDateRange::fromNative(test.native);
			CHECK_EQ(native2own, test.own);
		}
	}
}

TEST_CASE("[dpxdicom.dicomdate] Time can parse dicom")
{
	struct TestData // NOLINT
	{
		const char* input;
		bool parsedOk;
		DicomTime value = {};
		bool isNull = true;
	};

	static const TestData TEST_DATA[] = {
		{"", true},
		{"010203.004000", true, {1, 2, 3, 4}, false},
		{"010203.000004", true, {1, 2, 3, 0}, false},
		{"010203.00004", true, {1, 2, 3, 0}, false},
		{"010203.0004", true, {1, 2, 3, 0}, false},
		{"010203.004", true, {1, 2, 3, 4}, false},
		{"010203.04", true, {1, 2, 3, 40}, false},
		{"010203.4", true, {1, 2, 3, 400}, false},
		{"010203.", false},
		{"010203", true, {1, 2, 3}, false},
		{"01020", false},
		{"0102", true, {1, 2}, false},
		{"0102.3", false},
		{"010", false},
		{"01", true, {1}, false},
		{"01.2", false},
		{"0", false},
		{" 010203.4", false},
		{"010203.4 ", true, {1, 2, 3, 400}, false},
		{"010203.z", false},
		{"01020z.4", false},
		{"010z03.4", false},
		{"0z0203.4", false},
		{"+10203.4", false},
		{"-10203.4", false},
		{"01+203.4", false},
		{"01-203.4", false},
		{"0102+3.4", false},
		{"0102-3.4", false},
		{"010203.+4", false},
		{"010203.-4", false},
		{"010203z4", false},
		{"010203+4", false},
		{"010203-4", false},
	};

	for (const auto& test : TEST_DATA)
	{
		CAPTURE(test.input);
		DicomTime parsed;
		bool parsedOk = parsed.fromDicom(test.input, test.input + strlen(test.input));
		CHECK_EQ(parsedOk, test.parsedOk);
		CHECK_EQ(parsed, test.value);
		CHECK_EQ(parsed.isNull(), test.isNull);
	}
}

TEST_CASE("[dpxdicom.dicomdate] Time can write dicom")
{
	struct TestData // NOLINT
	{
		DicomTime input;
		const char* expected;
	};

	static const TestData TEST_DATA[] = {
		{{}, ""},
		{{1}, "01"},
		{{1, 2}, "0102"},
		{{1, 2, 3}, "010203"},
		{{1, 2, 3, 4}, "010203.004"},
		{{99, 99, 99, 999}, "999999.999"},
		{{100, 99, 99, 999}, ""},
		{{99, 100, 99, 999}, ""},
		{{99, 99, 100, 999}, ""},
		{{99, 99, 99, 1000}, ""},
	};

	for (const auto& test : TEST_DATA)
	{
		CAPTURE(test.input);
		QByteArray written;
		test.input.toDicom(written);
		CHECK_EQ(written, test.expected);
	}
}

TEST_CASE("[dpxdicom.dicomdate] Time can convert native")
{
	struct TestData // NOLINT
	{
		DicomTime own;
		QTime native;
		bool native2own = true;
		bool own2native = true;
	};

	static const TestData TEST_DATA[] = {
		{{}, {}, true, true},
		{{0, 0, 0, 0}, {0, 0, 0, 0}, false},
		{{0, 0, 0}, {0, 0, 0, 0}, true, false},
		{{1, 2, 3}, {1, 2, 3}},
		{{1, 2, 3, 4}, {1, 2, 3, 4}},
		{{23, 59, 59}, {23, 59, 59}},
		{{23, 59, 59, 999}, {23, 59, 59, 999}},
		{{1}, {1, 0, 0}, false},
		{{1, 2}, {1, 2, 0}, false},
	};

	static const auto fmt = [](const TestData& d)
	{
		std::ostringstream ss;
		ss << "TestData(" << d.own << ", " << d.native << ")";
		return ss.str();
	};

	for (const auto& test : TEST_DATA)
	{
		CAPTURE(fmt(test));
		if (test.own2native)
		{
			auto own2native = test.own.toNative();
			CHECK_EQ(own2native, test.native);
		}
		if (test.native2own)
		{
			auto native2own = DicomTime::fromNative(test.native);
			CHECK_EQ(native2own, test.own);
		}
	}
}

TEST_CASE("[dpxdicom.dicomdate] TimeRange can parse dicom")
{
	struct TestData // NOLINT
	{
		const char* input;
		bool parsedOk;
		DicomTimeRange value = {};
		bool isNull = true;
	};

	static const TestData TEST_DATA[] = {
		{"", true},
		{"010203", true, {{1, 2, 3}, {1, 2, 3}}, false},
		{"010203 ", true, {{1, 2, 3}, {1, 2, 3}}, false},
		{"-010203", true, {{}, {1, 2, 3}}, false},
		{"010203-", true, {{1, 2, 3}, {}}, false},
		{"0102-", true, {{1, 2}, {}}, false},
		{"01-", true, {{1}, {}}, false},
		{"01- ", true, {{1}, {}}, false},
		{"-", false},
		{"-01", true, {{}, {1}}, false},
		{"-0102", true, {{}, {1, 2}}, false},
		{"-010203", true, {{}, {1, 2, 3}}, false},
		{"-010203 ", true, {{}, {1, 2, 3}}, false},
		{"0z- ", false},
		{"-0z", false},
		{"-01z", false},
		{"01-02", true, {{1}, {2}}, false},
		{"010203-040506", true, {{1, 2, 3}, {4, 5, 6}}, false},
		{"010203.333-040506.444", true, {{1, 2, 3, 333}, {4, 5, 6, 444}}, false},
		{"010203-040506z", false},
		{" 010203-040506", false},
	};

	for (const auto& test : TEST_DATA)
	{
		CAPTURE(test.input);
		DicomTimeRange parsed;
		bool parsedOk = parsed.fromDicom(test.input, test.input + strlen(test.input));
		CHECK_EQ(parsedOk, test.parsedOk);
		CHECK_EQ(parsed, test.value);
		CHECK_EQ(parsed.isNull(), test.isNull);
	}
}

TEST_CASE("[dpxdicom.dicomdate] TimeRange can write dicom")
{
	struct TestData // NOLINT
	{
		DicomTimeRange input;
		const char* expected;
	};

	static const TestData TEST_DATA[] = {
		{{}, ""},
		{{{1, 2, 3, 4}, {5, 6, 7, 8}}, "010203.004-050607.008"},
		{{{1}, {1}}, "01-01"},
		{{{1}, {5}}, "01-05"},
		{{{1, 2}, {5}}, "0102-05"},
		{{{1}, {5, 6}}, "01-0506"},
		{{{1, 2, 3}, {5, 6, 7}}, "010203-050607"},
		{{{1}, {}}, "01-"},
		{{{}, {1}}, "-01"},
	};

	for (const auto& test : TEST_DATA)
	{
		CAPTURE(test.input);
		QByteArray written;
		test.input.toDicom(written);
		CHECK_EQ(written, test.expected);
	}
}

TEST_CASE("[dpxdicom.dicomdate] TimeRange can convert native")
{
	struct TestData // NOLINT
	{
		DicomTimeRange own;
		QPair<QTime, QTime> native;
		bool native2own = true;
		bool own2native = true;
	};

	static const TestData TEST_DATA[] = {
		{{}, {}, true, true},
		{{{1, 2, 3, 4}, {}}, {{1, 2, 3, 4}, {}}},
		{{{}, {1, 2, 3, 4}}, {{}, {1, 2, 3, 4}}},
		{{{1, 2, 3, 4}, {4, 5, 6, 7}}, {{1, 2, 3, 4}, {4, 5, 6, 7}}},
		{{{1, 2, 3, 4}, {1, 2, 3, 4}}, {{1, 2, 3, 4}, {1, 2, 3, 4}}},
		{{{1}, {2}}, {{1, 0, 0, 0}, {2, 59, 59, 999}}, false},
		{{{1, 2}, {2, 4}}, {{1, 2, 0}, {2, 4, 59, 999}}, false},
		{{{1, 2, 3}, {2, 4, 5}}, {{1, 2, 3, 0}, {2, 4, 5, 999}}, false},
	};

	static const auto fmt = [](const TestData& d)
	{
		std::ostringstream ss;
		ss << "TestData(" << d.own << ", " << d.native << ")";
		return ss.str();
	};

	for (const auto& test : TEST_DATA)
	{
		CAPTURE(fmt(test));
		if (test.own2native)
		{
			auto own2native = test.own.toNative();
			CHECK_EQ(own2native, test.native);
		}
		if (test.native2own)
		{
			auto native2own = DicomTimeRange::fromNative(test.native);
			CHECK_EQ(native2own, test.own);
		}
	}
}

TEST_CASE("[dpxdicom.dicomdate] DateTime can parse dicom")
{
	struct TestData // NOLINT
	{
		const char* input;
		bool isCFind;
		DicomTzOffset dsOffset;
		bool parsedOk;
		DicomDateTime value = {};
		bool isNull = true;
	};

	static const auto fmt = [](const TestData& d)
	{
		QByteArray rv = "TestData(\"";
		rv += d.input;
		rv += "\", C-FIND:";
		rv += (d.isCFind ? "t" : "f");
		rv += ", DatasetTZ:";
		d.dsOffset.toDicom(rv);
		rv += ")";
		return rv;
	};

	static const TestData TEST_DATA[] = {
		{"", false, {}, true},
		{"20010203040506+0300", false, {}, true, {{2001, 2, 3}, {4, 5, 6}, TZP(3, 0)}, false},
		{"20010203040506+0300", false, TZP(3, 0), true, {{2001, 2, 3}, {4, 5, 6}, TZP(3, 0)}, false},
		{"20010203040506", false, TZP(3, 0), true, {{2001, 2, 3}, {4, 5, 6}, {}}, false},
		{"20010203040506", false, TZP(2, 0), true, {{2001, 2, 3}, {4, 5, 6}, TZP(2, 0)}, false},
		{"00010203040506.007000+0809", false, {}, true, {{1, 2, 3}, {4, 5, 6, 7}, TZP(8, 9)}, false},
		{"00010203040506+0809", false, {}, true, {{1, 2, 3}, {4, 5, 6}, TZP(8, 9)}, false},
		{"000102030405+0809", false, {}, true, {{1, 2, 3}, {4, 5}, TZP(8, 9)}, false},
		{"0001020304+0809", false, {}, true, {{1, 2, 3}, {4}, TZP(8, 9)}, false},
		{"00010203+0809", false, {}, true, {{1, 2, 3}, {}, TZP(8, 9)}, false},
		{"000102+0809", false, {}, true, {{1, 2}, {}, TZP(8, 9)}, false},
		{"0001+0809", false, {}, true, {{1}, {}, TZP(8, 9)}, false},
		{"0001-0809", false, {}, true, {{1}, {}, TZM(8, 9)}, false},
		{"0001-0809", true, {}, false},
		{"0001", false, {}, true, {{1}, {}, {}}, false},
		{"0001", false, TZP(8, 9), true, {{1}, {}, TZP(8, 9)}, false},
		{"001", false, {}, false},
		{"0001 ", false, {}, true, {{1}, {}, {}}, false},
		{" 0001", false, {}, false},
	};

	auto old_default_tz = system_tz_offset_at_date;
	system_tz_offset_at_date = [](const QDateTime&) { return TZP(3, 0); };
	for (const auto& test : TEST_DATA)
	{
		CAPTURE(fmt(test));
		DicomDateTime parsed;
		bool parsedOk = parsed.fromDicom(test.input, test.input + strlen(test.input), test.isCFind, test.dsOffset);
		CHECK_EQ(parsedOk, test.parsedOk);
		CHECK_EQ(parsed, test.value);
		CHECK_EQ(parsed.isNull(), test.isNull);
	}
	system_tz_offset_at_date = old_default_tz;
}

TEST_CASE("[dpxdicom.dicomdate] DateTime can write dicom")
{
	struct TestData // NOLINT
	{
		DicomDateTime input;
		bool isCFind;
		DicomTzOffset dsOffset;
		const char* expected;
	};

	static const TestData TEST_DATA[] = {
		{{}, false, {}, ""},
		{{{}, {1}}, false, {}, ""},
		{{{}, {}, {0}}, false, {}, ""},
		{{{1}}, false, {}, "0001"},
		{{{1, 2}}, false, {}, "000102"},
		{{{1, 2, 3}}, false, {}, "00010203"},
		{{{1, 2, 3}, {4}}, false, {}, "0001020304"},
		{{{1, 2, 3}, {4, 5}}, false, {}, "000102030405"},
		{{{1, 2, 3}, {4, 5, 6}}, false, {}, "00010203040506"},
		{{{1, 2, 3}, {4, 5, 6, 7}}, false, {}, "00010203040506.007"},
		{{{2001, 2, 3}, {4, 5, 6}}, false, TZP(3, 0), "20010203040506"},
		{{{2001, 2, 3}, {4, 5, 6}}, false, TZP(2, 0), "20010203040506+0200"},
		{{{2001, 2, 3}, {4, 5, 6}, TZP(3, 0)}, false, {}, "20010203040506+0300"},
		{{{2001, 2, 3}, {4, 5, 6}, TZP(3, 0)}, false, TZP(3, 0), "20010203040506"},
		{{{2001, 2, 3}, {4, 5, 6}, TZP(3, 0)}, false, TZP(2, 0), "20010203040506+0300"},
		{{{2001, 2, 3}, {4, 5, 6}, TZP(2, 0)}, false, TZP(3, 0), "20010203040506+0200"},
		{{{2001, 2, 3}, {4, 5, 6}, TZP(2, 0)}, false, TZP(2, 0), "20010203040506"},
		{{{2001, 2, 3}, {4, 5, 6}, TZM(1, 0)}, false, {}, "20010203040506-0100"},
		{{{2001, 2, 3}, {4, 5, 6}, TZM(1, 0)}, true, {}, "20010203050506+0000"},
	};

	static const auto fmt = [](const TestData& d)
	{
		std::ostringstream ss;
		ss << "TestData(" << d.input << ", C-FIND:" << (d.isCFind ? 't' : 'f') << ", DatasetTZ:" << d.dsOffset << ")";
		return ss.str();
	};

	auto old_default_tz = system_tz_offset_at_date;
	system_tz_offset_at_date = [](const QDateTime&) { return TZP(3, 0); };
	for (const auto& test : TEST_DATA)
	{
		CAPTURE(fmt(test));
		QByteArray written;
		test.input.toDicom(written, test.isCFind, false, test.dsOffset);
		CHECK_EQ(written, test.expected);
	}
	system_tz_offset_at_date = old_default_tz;
}

TEST_CASE("[dpxdicom.dicomdate] DateTime can convert native")
{
	struct TestData // NOLINT
	{
		DicomDateTime own;
		QDateTime native;
		bool native2own = true;
		bool own2native = true;
	};

	static const TestData TEST_DATA[] = {
		{{}, {}, true, true},
		{{{2001, 2, 3}, {4, 5, 6, 7}, TZP(8, 9)}, {{2001, 2, 3}, {4, 5, 6, 7}, Qt::OffsetFromUTC, TZP(8, 9).seconds}},
		{{{2001, 2, 3}, {4, 5, 6, 7}, {}}, {{2001, 2, 3}, {4, 5, 6, 7}}},
	};

	static const auto fmt = [](const TestData& d)
	{
		std::ostringstream ss;
		ss << "TestData(" << d.own << ", " << d.native << ")";
		return ss.str();
	};

	for (const auto& test : TEST_DATA)
	{
		CAPTURE(fmt(test));
		if (test.own2native)
		{
			auto own2native = test.own.toNative();
			CHECK_EQ(own2native, test.native);
		}
		if (test.native2own)
		{
			auto native2own = DicomDateTime::fromNative(test.native);
			CHECK_EQ(native2own, test.own);
		}
	}
}

TEST_CASE("[dpxdicom.dicomdate] DateTimeRange can parse dicom")
{
	struct TestData // NOLINT
	{
		const char* input;
		DicomTzOffset dsOffset;
		bool parsedOk;
		DicomDateTimeRange value = {};
		bool isNull = true;
	};

	static const TestData TEST_DATA[] = {
		{"", {}, true},
		{"20010203", {}, true, {{{2001, 2, 3}, {}, {}}, {{2001, 2, 3}, {}, {}}}, false},
		{"20010203 ", {}, true, {{{2001, 2, 3}, {}, {}}, {{2001, 2, 3}, {}, {}}}, false},
		{"-20010203", {}, true, {{}, {{2001, 2, 3}, {}, {}}}, false},
		{"20010203040506.007+0809-", {}, true, {{{2001, 2, 3}, {4, 5, 6, 7}, TZP(8, 9)}, {}}, false},
		{"20010203040506.007-", {}, true, {{{2001, 2, 3}, {4, 5, 6, 7}, {}}, {}}, false},
		{"20010203040506.007-", TZP(8, 9), true, {{{2001, 2, 3}, {4, 5, 6, 7}, TZP(8, 9)}, {}}, false},
		{"20010203040506-", {}, true, {{{2001, 2, 3}, {4, 5, 6}, {}}, {}}, false},
		{"200102030405-", {}, true, {{{2001, 2, 3}, {4, 5}, {}}, {}}, false},
		{"2001020304-", {}, true, {{{2001, 2, 3}, {4}, {}}, {}}, false},
		{"20010203-", {}, true, {{{2001, 2, 3}, {}, {}}, {}}, false},
		{"200102-", {}, true, {{{2001, 2}, {}, {}}, {}}, false},
		{"2001-", {}, true, {{{2001}, {}, {}}, {}}, false},
		{"2001- ", {}, true, {{{2001}, {}, {}}, {}}, false},
		{"-", {}, false},
		{"-2001", {}, true, {{}, {{2001}, {}, {}}}, false},
		{"-200102", {}, true, {{}, {{2001, 2}}}, false},
		{"-20010203", {}, true, {{}, {{2001, 2, 3}}}, false},
		{"-2001020304", {}, true, {{}, {{2001, 2, 3}, {4}}}, false},
		{"-200102030405", {}, true, {{}, {{2001, 2, 3}, {4, 5}}}, false},
		{"-20010203040506", {}, true, {{}, {{2001, 2, 3}, {4, 5, 6}}}, false},
		{"-20010203040506.007", {}, true, {{}, {{2001, 2, 3}, {4, 5, 6, 7}}}, false},
		{"-20010203040506.007+0809", {}, true, {{}, {{2001, 2, 3}, {4, 5, 6, 7}, TZP(8, 9)}}, false},
	};

	static const auto fmt = [](const TestData& d)
	{
		QByteArray rv = "TestData(\"";
		rv += d.input;
		rv += ", DatasetTZ:";
		d.dsOffset.toDicom(rv);
		rv += ")";
		return rv;
	};

	for (const auto& test : TEST_DATA)
	{
		CAPTURE(fmt(test));
		DicomDateTimeRange parsed;
		bool parsedOk = parsed.fromDicom(test.input, test.input + strlen(test.input), test.dsOffset);
		CHECK_EQ(parsedOk, test.parsedOk);
		CHECK_EQ(parsed, test.value);
		CHECK_EQ(parsed.isNull(), test.isNull);
	}
}

TEST_CASE("[dpxdicom.dicomdate] DateTimeRange can write dicom")
{
	struct TestData // NOLINT
	{
		DicomDateTimeRange input;
		DicomTzOffset dsOffset;
		const char* expected;
	};

	static const TestData TEST_DATA[] = {
		{{}, {}, ""},
		{{{{2001}}, {}}, {}, "2001-"},
		{{{{2001}, {2}}, {}}, {}, "2001010102-"},
		{{{}, {{2001}}}, {}, "-2001"},
		{{{{2001}}, {{2001}}}, {}, "2001-2001"},
		{{{{2001, 2, 3}, {4, 5, 6, 7}, TZP(8, 9)}, {{2001, 2, 3}, {4, 5, 6, 7}, TZP(8, 9)}},
		 {},
		 "20010203040506.007+0809-20010203040506.007+0809"},
		{{{{2001, 2, 3}, {4, 5, 6}, TZP(1, 2)}, {}}, {}, "20010203040506+0102-"},
		{{{{2001, 2, 3}, {4, 5, 6}, TZM(1, 2)}, {}}, {}, "20010203050706+0000-"},
	};

	static const auto fmt = [](const TestData& d)
	{
		std::ostringstream ss;
		ss << "TestData(" << d.input << ", DatasetTZ:" << d.dsOffset << ")";
		return ss.str();
	};

	for (const auto& test : TEST_DATA)
	{
		CAPTURE(fmt(test));
		QByteArray written;
		test.input.toDicom(written, false, test.dsOffset);
		CHECK_EQ(written, test.expected);
	}
}

TEST_CASE("[dpxdicom.dicomdate] DateTimeRange can convert native")
{
	struct TestData // NOLINT
	{
		DicomDateTimeRange own;
		QPair<QDateTime, QDateTime> native;
		bool native2own = true;
		bool own2native = true;
	};

	static const TestData TEST_DATA[] = {
		{{}, {}},
		{{{{1, 2, 3}, {4, 5, 6, 7}, TZP(8, 9)}, {}},
		 {{{1, 2, 3}, {4, 5, 6, 7}, Qt::OffsetFromUTC, TZP(8, 9).seconds}, {}}},
		{{{{2001, 2}}, {{2002, 4}}}, {{{2001, 2, 1}, {0, 0, 0}}, {{2002, 4, 30}, {23, 59, 59, 999}}}, false},
	};

	static const auto fmt = [](const TestData& d)
	{
		std::ostringstream ss;
		ss << "TestData(" << d.own << ", " << d.native << ")";
		return ss.str();
	};

	for (const auto& test : TEST_DATA)
	{
		CAPTURE(fmt(test));
		if (test.own2native)
		{
			auto own2native = test.own.toNative();
			CHECK_EQ(own2native, test.native);
		}
		if (test.native2own)
		{
			auto native2own = DicomDateTimeRange::fromNative(test.native);
			CHECK_EQ(native2own, test.own);
		}
	}
}

} // namespace tests

#endif
