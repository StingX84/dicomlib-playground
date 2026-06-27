//////////////////////////////////////////////////////////////////////////////
/// \file dpxdicom/data/dicomdate.h
/// \brief Файл описания методов обработки текстовых строк в значениях атрибутов
/// \author Девятников А.В.
/// \date 2025-03-27
///
/// Copyright (C) 2025 by RTK Radiology
/// ALL RIGHTS RESERVED.
//////////////////////////////////////////////////////////////////////////////

#pragma once

#include "dpxdicom/dicomlib.h"

#include <limits>

class QDate;
class QTime;
class QDateTime;

struct DicomDate;

/** Структура, содержащая оффсет времени от UTC в секундах.
 *
 * Это смещение в стандарте используется в двух местах:
 * 1. В качестве значения атрибута Timezone Offset From UTC (0008,0201)
 * 2. В качестве опционального суффикса к значению атрибута с типом "DT"
 *
 * Текстовое представление идентично в обоих случаях.
 *
 * Выдержка из стандарта, относительно использования в атрибуте "DT", а также
 * ограничений на диапазон значения см. в #DateTime.
 *
 * DICOM PS3.3 C.12.1.1.8 Timezone Offset From UTC
 *
 * \quotation
 * Encoded as an ASCII string in the format "&ZZXX". The components of this
 * string, from left to right, are & = "+" or "-", and ZZ = Hours and XX =
 * Minutes of offset. Leading space characters shall not be present.
 *
 * The offset for UTC shall be +0000; -0000 shall not be used.
 *
 * Note
 * 1. This encoding is the same as described in PS3.5 for the offset component
 * of the DT Value Representation.
 * 2. This Attribute does not apply to values with a DT Value Representation,
 * that contains an explicitly encoded timezone offset.
 * 3. The corrected time may cross a 24 hour boundary. For example, if Local
 * Time = 1.00 a.m. and Offset = +0200, then UTC = 11.00 p.m. (23.00) the day
 * before.
 * 4. The "+" sign may not be omitted.
 *
 * Time earlier than UTC is expressed as a negative offset.
 * \endquotation
 */
struct DicomTzOffset
{
	/// Константа, указывающая на невалидное/пустое значение Time Offset
	static constexpr std::int32_t Unset = std::numeric_limits<std::int32_t>::max();

	/// Минимальное значенике (задано стандартом. см. #DateTime)
	static constexpr std::int32_t Min = -12 * 3600;

	/// Максимальное значенике (задано стандартом. см. #DateTime)
	static constexpr std::int32_t Max = 14 * 3600;

	std::int32_t seconds = Unset;

	inline constexpr bool isNull() const { return seconds == Unset; }

	inline constexpr bool isValid() const { return seconds >= Min && seconds <= Max; }

	inline constexpr bool isNegative() const { return !isNull() && seconds < 0; }

	CAP_DICOMLIB_EXPORT static DicomTzOffset system(const QDateTime& atDate) noexcept;
	CAP_DICOMLIB_EXPORT static DicomTzOffset system(const DicomDate& atDate) noexcept;
	CAP_DICOMLIB_EXPORT static DicomTzOffset system() noexcept;

	CAP_DICOMLIB_EXPORT void toDicom(QByteArray& target) const noexcept;
	CAP_DICOMLIB_EXPORT bool fromDicom(const char* p, const char* pstop) noexcept;

	constexpr bool operator== (const DicomTzOffset& o) const { return seconds == o.seconds; };
};

/** Структура, содержащая дату в обычном или C-FIND-RQ датасете (тип атрибута "DA")
 *
 * Документация из стандарта (PS3.5 Table 6.2-1. DICOM Value Representations):
 *
 * DA Date
 *
 * \quotation
 * A string of characters of the format YYYYMMDD; where YYYY shall contain year,
 * MM shall contain the month, and DD shall contain the day, interpreted as a
 * date of the Gregorian calendar system.
 *
 * Example:
 * "19930822" would represent August 22, 1993.
 *
 * Note
 * The ACR-NEMA Standard 300 (predecessor to DICOM) supported a string of
 * characters of the format YYYY.MM.DD for this VR. Use of this format is not
 * compliant.
 *
 * See also DT VR in this table.
 *
 * Dates before year 1582, e.g., used for dating historical or archeological
 * items, are interpreted as proleptic Gregorian calendar dates, unless
 * otherwise specified.
 *
 * Alternatively, in the context of a Query with Empty Value Matching (see PS3.4),
 * a string of two QUOTATION MARK characters, representing an empty key Value.
 * \endquotation
 */
struct DicomDate
{
	using Native = QDate;

	std::uint16_t y = std::numeric_limits<uint16_t>::max();
	std::uint8_t m = std::numeric_limits<uint8_t>::max();
	std::uint8_t d = std::numeric_limits<uint8_t>::max();

	inline constexpr bool isNull() const { return y == std::numeric_limits<uint16_t>::max(); }

	inline constexpr bool isAllFieldsSet() const
	{
		return y != std::numeric_limits<uint16_t>::max() && m != std::numeric_limits<uint8_t>::max()
			&& d != std::numeric_limits<uint8_t>::max();
	}

	CAP_DICOMLIB_EXPORT DicomDate minimized() const noexcept;
	CAP_DICOMLIB_EXPORT DicomDate maximized() const noexcept;

	CAP_DICOMLIB_EXPORT QDate toNative() const noexcept;
	CAP_DICOMLIB_EXPORT static DicomDate fromNative(const QDate& value) noexcept;

	CAP_DICOMLIB_EXPORT void toDicom(QByteArray& target) const noexcept;
	CAP_DICOMLIB_EXPORT bool fromDicom(const char* p, const char* pstop) noexcept;

	constexpr bool operator== (const DicomDate& r) const { return y == r.y && m == r.m && d == r.d; };
};

/** Структура, содержащая диапазон дат в C-FIND-RQ датасете (тип атрибута "DA")
 *
 * см. #Date для базового описания. Дополнительно в стандарте про
 * "Range Matching" (PS3.5 Table 6.2-1. DICOM Value Representations):
 *
 * DA Date
 *
 * \quotation
 * In the context of a Query with Range Matching (see PS3.4), the character "-"
 * is allowed, and a trailing SPACE character is allowed for padding.
 *
 * In the context of a Query with Empty Value Matching (see PS3.4), the
 * QUOTATION MARK character is allowed.
 *
 * Из стандарта PS3.4 C.2.2.2.5.1 Range Matching of Attributes of VR of DA:
 *
 * In the absence of Extended Negotiation, then:
 * a. A string of the form "<date1> - <date2>", where <date1> is less or equal
 *    to <date2>, shall match all occurrences of dates that fall between <date1>
 *    and <date2> inclusive
 * b. A string of the form "- <date1>" shall match all occurrences of dates
 *    prior to and including <date1>
 * c. A string of the form "<date1> -" shall match all occurrences of <date1>
 *    and subsequent dates
 * \endquotation
 *
 * При конвертации в QDate происходят следующие трансформации:
 * - если from задан, то все незаданные поля возвращаются в неименьшем виде.
 *   Например, если входной текст "200102", то будет возвращен `QDate(2001, 01, 01)`
 * - если to задан, то все незаданные поля возвращаются в наибольшем виде.
 *   Например, если входной текст "200102", то будет возвращен `QDate(2001, 01, 31)`
 */
struct DicomDateRange
{
	using Native = QPair<QDate, QDate>;

	DicomDate from;
	DicomDate to;

	inline constexpr bool isNull() const { return from.isNull() && to.isNull(); }

	CAP_DICOMLIB_EXPORT QPair<QDate, QDate> toNative() const noexcept;
	CAP_DICOMLIB_EXPORT static DicomDateRange fromNative(const QPair<QDate, QDate>& value) noexcept;

	CAP_DICOMLIB_EXPORT void toDicom(QByteArray& target) const noexcept;
	CAP_DICOMLIB_EXPORT bool fromDicom(const char* p, const char* pstop) noexcept;

	constexpr bool operator== (const DicomDateRange& r) const { return from == r.from && to == r.to; };
};

/** Структура, содержащая время в обычном или C-FIND-RQ датасете (тип атрибута "TM")
 *
 * Документация из стандарта (PS3.5 Table 6.2-1. DICOM Value Representations):
 *
 * TM Time
 *
 * \quotation
 * A string of characters of the format HHMMSS.FFFFFF; where HH contains hours
 * (range "00" - "23"), MM contains minutes (range "00" - "59"), SS contains
 * seconds (range "00" - "60"), and FFFFFF contains a fractional part of a
 * second as small as 1 millionth of a second (range "000000" - "999999"). A
 * 24-hour clock is used. Midnight shall be represented by only "0000" since
 * "2400" would violate the hour range. The string may be padded with trailing
 * spaces. Leading and embedded spaces are not allowed.
 *
 * One or more of the components MM, SS, or FFFFFF may be unspecified as long
 * as every component to the right of an unspecified component is also
 * unspecified, which indicates that the Value is not precise to the precision
 * of those unspecified components.
 *
 * The FFFFFF component, if present, shall contain 1 to 6 digits. If FFFFFF is
 * unspecified the preceding "." shall not be included.
 *
 * Examples:
 * 1. "070907.0705 " represents a time of 7 hours, 9 minutes and 7.0705 seconds.
 * 2. "1010" represents a time of 10 hours, and 10 minutes.
 * 3. "021 " is an invalid Value.
 *
 * Note
 * 1. The ACR-NEMA Standard 300 (predecessor to DICOM) supported a string of
 *    characters of the format HH:MM:SS.frac for this VR. Use of this format is
 *    not compliant.
 * 2. See also DT VR in this table.
 * 3. The SS component may have a Value of 60 only for a leap second.
 *
 * Alternatively, in the context of a Query with Empty Value Matching (see
 * PS3.4), a string of two QUOTATION MARK characters, representing an empty key
 * Value.
 * \endquotation
 *
 * Особенности:
 * - При чтении, "leap" секунда со значением "60" превращается в "59".
 * - Запись фракции секунды в текст происходит только, если она отлична от 0.
 *   Записываются только три цифры, т.к. внутреннее разрешение в миллисекундах.
 */
struct DicomTime
{
	using Native = QTime;

	std::uint8_t h = std::numeric_limits<uint8_t>::max();
	std::uint8_t m = std::numeric_limits<uint8_t>::max();
	std::uint8_t s = std::numeric_limits<uint8_t>::max();
	std::uint16_t ms = std::numeric_limits<uint16_t>::max();

	inline constexpr bool isNull() const { return h == std::numeric_limits<uint8_t>::max(); }

	inline constexpr bool isAllFieldsSet() const
	{
		return h != std::numeric_limits<uint8_t>::max() && m != std::numeric_limits<uint8_t>::max()
			&& s != std::numeric_limits<uint8_t>::max() && ms != std::numeric_limits<uint16_t>::max();
	}

	CAP_DICOMLIB_EXPORT DicomTime minimized() const noexcept;
	CAP_DICOMLIB_EXPORT DicomTime maximized() const noexcept;

	CAP_DICOMLIB_EXPORT QTime toNative() const noexcept;
	CAP_DICOMLIB_EXPORT static DicomTime fromNative(const QTime& value) noexcept;

	CAP_DICOMLIB_EXPORT void toDicom(QByteArray& target) const noexcept;
	CAP_DICOMLIB_EXPORT bool fromDicom(const char* p, const char* pstop) noexcept;

	constexpr bool operator== (const DicomTime& r) const { return h == r.h && m == r.m && s == r.s && ms == r.ms; };
};

/** Структура, содержащая время в C-FIND-RQ датасете (тип атрибута "TM")
 *
 *  см. #Time для базового описания. Дополнительно в стандарте про
 * "Range Matching" (PS3.5 Table 6.2-1. DICOM Value Representations):
 *
 * TM Time
 *
 * \quotation
 * In the context of a Query with Range Matching (see PS3.4), the character "-"
 * is allowed, and a trailing SPACE character is allowed for padding.
 *
 * In the context of a Query with Empty Value Matching (see PS3.4), the
 * QUOTATION MARK character is allowed.
 * \endquotation
 *
 * Из стандарта PS3.4 C.2.2.2.5.2 Range Matching of Attributes of VR of TM:
 *
 * \quotation
 * All comparison specified in the following shall be based on a direct
 * comparison of times within a day. "Prior" includes all times starting from
 * midnight of the same day to the specified time. "Subsequent" includes all
 * times starting with the specified time until any time prior to midnight of
 * the following day. Range Matching crossing midnight is not supported.
 *
 * No offset from Universal Coordinated Time is permitted in the TM VR values.
 * If Timezone Offset From UTC (0008,0201) is present in the query identifier,
 * the specified time values and the definition of midnight are in the
 * specified timezone.
 *
 * In the absence of Extended Negotiation, then:
 *
 * a. A string of the form "<time1> - <time2>", where <time1> is less or equal
 *    to <time2>, shall match all occurrences of times that fall between <time1>
 *    and <time2> inclusive
 * b. A string of the form "- <time1>" shall match all occurrences of times
 *    prior to and including <time1>
 * c. A string of the form "<time1> -" shall match all occurrences of <time1>
 *    and subsequent times
 * \endquotation
 *
 * При конвертации в QTime происходят следующие трансформации:
 * - если from задан, то все незаданные поля возвращаются в неименьшем виде.
 *   Например, если входной текст "2301", то будет возвращен `QTime(23, 1, 0, 0)`
 * - если to задан, то все незаданные поля возвращаются в наибольшем виде.
 *   Например, если входной текст "2301", то будет возвращен `QTime(23, 1, 59, 999)`
 */
struct DicomTimeRange
{
	using Native = QPair<QTime, QTime>;

	DicomTime from;
	DicomTime to;

	inline constexpr bool isNull() const { return from.isNull() && to.isNull(); }

	CAP_DICOMLIB_EXPORT QPair<QTime, QTime> toNative() const noexcept;
	CAP_DICOMLIB_EXPORT static DicomTimeRange fromNative(const QPair<QTime, QTime>& value) noexcept;

	CAP_DICOMLIB_EXPORT void toDicom(QByteArray& target) const noexcept;
	CAP_DICOMLIB_EXPORT bool fromDicom(const char* p, const char* pstop) noexcept;

	constexpr bool operator== (const DicomTimeRange& r) const { return from == r.from && to == r.to; };
};

/** Структура, содержащая дату-время в обычном или C-FIND-RQ датасете (тип атрибута "DT")
 *
 * Документация из стандарта (PS3.5 Table 6.2-1. DICOM Value Representations):
 *
 * DT Date Time
 *
 * \quotation
 * A concatenated date-time character string in the format:
 *
 * YYYYMMDDHHMMSS.FFFFFF&ZZXX
 *
 * The components of this string, from left to right, are YYYY = Year, MM =
 * Month, DD = Day, HH = Hour (range "00" - "23"), MM = Minute (range "00" -
 * "59"), SS = Second (range "00" - "60").
 *
 * FFFFFF = Fractional Second contains a fractional part of a second as small
 * as 1 millionth of a second (range "000000" - "999999").
 *
 * &ZZXX is an optional suffix for offset from Coordinated Universal Time
 * (UTC), where & = "+" or "-", and ZZ = Hours and XX = Minutes of offset.
 *
 * The year, month, and day shall be interpreted as a date of the Gregorian
 * calendar system.
 *
 * A 24-hour clock is used. Midnight shall be represented by only "0000" since
 * "2400" would violate the hour range.
 *
 * The Fractional Second component, if present, shall contain 1 to 6 digits. If
 * Fractional Second is unspecified the preceding "." shall not be included.
 * The offset suffix, if present, shall contain 4 digits. The string may be
 * padded with trailing SPACE characters. Leading and embedded spaces are not
 * allowed.
 *
 * A component that is omitted from the string is termed a null component.
 * Trailing null components of Date Time indicate that the Value is not precise
 * to the precision of those components. The YYYY component shall not be null.
 * Non-trailing null components are prohibited. The optional suffix is not
 * considered as a component.
 *
 * A Date Time Value without the optional suffix is interpreted to be in the
 * local time zone of the application creating the Data Element, unless
 * explicitly specified by the Timezone Offset From UTC (0008,0201).
 *
 * UTC offsets are calculated as "local time minus UTC". The offset for a Date
 * Time Value in UTC shall be +0000.
 *
 * Alternatively, in the context of a Query with Empty Value Matching (see
 * PS3.4), a string of two QUOTATION MARK characters, representing an empty key
 * Value.
 *
 * Note
 *
 * 1. The range of the offset is -1200 to +1400. The offset for United States
 *    Eastern Standard Time is -0500. The offset for Japan Standard Time is
 *    +0900.
 * 2. The RFC 2822 use of -0000 as an offset to indicate local time is not
 *    allowed.
 * 3. A Date Time Value of 195308 means August 1953, not specific to particular
 *    day. A Date Time Value of 19530827111300.0 means August 27, 1953, 11;13
 *    a.m. accurate to 1/10th second.
 * 4. The Second component may have a Value of 60 only for a leap second.
 * 5. The offset may be included regardless of null components; e.g., 2007-0500
 *    is a legal Value.
 * \endquotation
 *
 * Особенности записи DT для C-FIND:
 *
 * DICOM PS3.4  C.2.2.2 Attribute Matching:
 *
 * \quotation
 * Note:
 * 1. For example, the "-" character is not valid for the DA, DT and TM VRs but
 * is used for Range Matching.
 *
 * DICOM PS3.4 C.2.2.2.1 Single Value Matching:
 *
 * b. of VR of DA, TM or DT and contains a single value with no "-" and no
 * QUOTATION MARK characters, or
 * \endquotation
 *
 * DICOM PS3.4 C.2.2.2.1.3 Attributes of VR of DA, DT or TM
 *
 * \quotation
 * Note:
 *
 * 3. Exclusion of the "-" character for Single Value Matching implies that a
 * Key Attribute with a VR of DT may not contain a negative offset from
 * Universal Coordinated Time (UTC) if Single Value Matching is intended. Use
 * of the "-" character in values of VR of TM, DA and DT indicates Range
 * Matching.
 *
 * 4. If an application is in a local time zone that has a negative offset then
 * it cannot perform Single Value Matching using a local time notation.
 * Instead, it can convert the Key Attribute Value to UTC and use an explicit
 * suffix of "+0000".
 * \endquotation
 *
 * Особенности:
 * - При чтении, "leap" секунда со значением "60" превращается в "59".
 * - Запись фракции секунды в текст происходит только, если она отлична от 0.
 *   Записываются только три цифры, т.к. внутреннее разрешение в миллисекундах.
 * - Чтение смещения времени из текста:
 *   - Если в строке не указано смещение, то принимается смещение из датасета.
 * - Запись смещения времени в текст:
 *   - Если #tzOffset был получен из датасета, то оффсет не записывается. При этом,
 *     Дата/время приводятся к текущему смещению в датасете, если необходимо.
 *   - Если #tzOffset должен быть записан, но он меньше нуля в случае записи C-FIND,
 *     то дата/время приводятся к временной зоне UTC и записываются со смещением "+0000".
 */
struct DicomDateTime
{
	using Native = QDateTime;

	DicomDate date;			  ///< Компонент "дата" (может содержать только год)
	DicomTime time;			  ///< Компонент "время" (может вообще не содержать ни одного компонента)
	DicomTzOffset tzOffset;	  ///< Компонент "смещение от UTC" (может быть пустым, если системное)
	bool tzSetFromDataset {}; ///< Был ли компонент #tzOffset получен из датасета в #fromDicom.

	inline constexpr bool isNull() const { return date.isNull(); }

	inline constexpr bool isAllFieldsSet() const { return date.isAllFieldsSet() && time.isAllFieldsSet(); }

	CAP_DICOMLIB_EXPORT DicomDateTime minimized() const noexcept;
	CAP_DICOMLIB_EXPORT DicomDateTime maximized() const noexcept;

	CAP_DICOMLIB_EXPORT QDateTime toNative() const noexcept;
	CAP_DICOMLIB_EXPORT static DicomDateTime fromNative(const QDateTime& value) noexcept;

	CAP_DICOMLIB_EXPORT void toDicom(QByteArray& target, bool isCFindRq = false, bool alwaysWriteOffset = false,
									 DicomTzOffset offsetInDataset = {}) const noexcept;
	CAP_DICOMLIB_EXPORT bool fromDicom(const char* p, const char* pstop, bool isCFindRq = false,
									   DicomTzOffset offsetInDataset = {}) noexcept;

	constexpr bool operator== (const DicomDateTime& r) const
	{
		return date == r.date && time == r.time && tzOffset == r.tzOffset;
	};

	/** Возвращает объект с применением изменений во временной зоне датасета
	 *
	 * Функция также учитывает специфику записи временной зоны в случае C-FIND. А именно: отрицательная зона
	 * превращается в UTC. См. DICOM PS3.4 C.2.2.2.1.3 Attributes of VR of DA, DT or TM
	 *
	 * \param isCFindRq Является ли дата поисковым ключем в C-FIND-RQ
	 * \param alwaysWriteOffset Флаг, требующий, чтобы в выходной структуре всегда была установлено
	 * смещение, даже если оно локальное.
	 * \param offsetInDataset Текущее смещение во времени у датасета
	 * \param[out] rvBecomesMoreSpecific (опционально)Возвращает признак того, что возращенный объект стал
	 * более специфичным, чем текущий. Например, в текущем поле #m было не заполнено, а в выходном
	 * оно стало заполнено. В случае получения этого флага, нужно повторить вызов этого метода для #minimized
	 * и #maximized вариантов с формированием #DicomDateTimeRange.
	 * \param[out] rvOffsetWriteRequired (опционально) Возвращает признак того, что запись оффсета необходима.
	 */
	CAP_DICOMLIB_EXPORT DicomDateTime adjustToNewDatasetOffset(bool isCFindRq, bool alwaysWriteOffset,
															   DicomTzOffset offsetInDataset,
															   bool *rvBecomesMoreSpecific = nullptr,
															   bool *rvOffsetWriteRequired = nullptr) const noexcept;
};

/** Структура, содержащая дату-время в C-FIND-RQ датасете (тип атрибута "DT")
 *
 *  см. #DicomDateTime для базового описания. Дополнительно в стандарте про
 * "Range Matching" (PS3.5 Table 6.2-1. DICOM Value Representations):
 *
 * DT Date Time
 *
 * \quotation
 * In the context of a Query with Empty Value Matching (see PS3.4), the
 * QUOTATION MARK character is allowed.
 *
 * Из стандарта PS3.4 C.2.2.2.5.3 Range Matching of Attributes of VR of DT:
 *
 * a. A string of the form "<datetime1> - <datetime2>", where <datetime1> is
 *    less or equal to <datetime2>, shall match all moments in time that fall
 *    between <datetime1> and <datetime2> inclusive
 *
 * b. A string of the form "- <datetime1>" shall match all moments in time
 *    prior to and including <datetime1>
 *
 * c. A string of the form "<datetime1> -" shall match all moments in time
 *    subsequent to and including <datetime1>
 *
 * d. The offset from Universal Coordinated Time, if present in the Value of
 *    the Attribute, shall be taken into account for the purposes of the match.
 * \endquotation
 *
 * При конвертации в QDateTime происходят следующие трансформации:
 * - если from задан, то все незаданные поля возвращаются в неименьшем виде.
 * - если to задан, то все незаданные поля возвращаются в наибольшем виде.
 */
struct DicomDateTimeRange
{
	using Native = QPair<QDateTime, QDateTime>;

	DicomDateTime from;
	DicomDateTime to;

	inline constexpr bool isNull() const { return from.isNull() && to.isNull(); }

	CAP_DICOMLIB_EXPORT QPair<QDateTime, QDateTime> toNative() const noexcept;
	CAP_DICOMLIB_EXPORT static DicomDateTimeRange fromNative(const QPair<QDateTime, QDateTime>& value) noexcept;

	CAP_DICOMLIB_EXPORT void toDicom(QByteArray& target, bool alwaysWriteOffset = false, DicomTzOffset offsetInDataset = {}) const noexcept;
	CAP_DICOMLIB_EXPORT bool fromDicom(const char* p, const char* pstop, DicomTzOffset offsetInDataset = {}) noexcept;

	constexpr bool operator== (const DicomDateTimeRange& r) const { return from == r.from && to == r.to; };
};

/** Структура, содержащая диапазон поиска числовых значений в C-FIND-RQ датасете (тип атрибута "IS")
 *
 * Стандарт поддерживает поиск по диапазону только для VR = DA, DT и TM, но, в частном порядке, эта реализация
 * поддерживает также поиск по диапазону числовых значений с VR = IS.
 *
 * Такой поиск в порядке исключения используется для атрибута #PR_TAG_STUDY_PATIENT_AGE. И является результатом разбора
 * атрибута #PR_TAG_QR_RANGE_FIND в классе #CDatasetDbMapper.
 *
 * Класс #CDatasetBase самостоятельно не работает с таким типом данных ни в #QVariant, ни в #QJsonValue. Но возможно
 * его использование, например, с кастомным #CDatasetBase::CustomAttributeProcessor. Например, PACS сервер регистрирует
 * специальный обработчик для атрибута #PR_TAG_STUDY_PATIENT_AGE, который может читать/писать типы `int` и
 * `DicomIntRange<int>`
 */
template<class TInt> struct DicomFindIntRange
{
	using Native = QPair<TInt, TInt>;

	TInt from = std::numeric_limits<TInt>::max();
	TInt to = std::numeric_limits<TInt>::max();

	inline constexpr bool isNull() const
	{
		return from == std::numeric_limits<TInt>::max() && to == std::numeric_limits<TInt>::max();
	}

	CAP_DICOMLIB_EXPORT Native toNative() const noexcept;
	CAP_DICOMLIB_EXPORT static DicomFindIntRange fromNative(const Native& value) noexcept;

	CAP_DICOMLIB_EXPORT void toDicom(QByteArray& target) const noexcept;
	CAP_DICOMLIB_EXPORT bool fromDicom(const char* p, const char* pstop) noexcept;

	constexpr bool operator== (const DicomFindIntRange& r) const { return from == r.from && to == r.to; };
};

namespace details
{
template<class T> static constexpr bool date_field_is_set(T x) noexcept
{
	return x != std::numeric_limits<T>::max();
}

template<class T> static constexpr void date_field_clear(T& x) noexcept
{
	x = std::numeric_limits<T>::max();
}
}

Q_DECLARE_METATYPE(DicomTzOffset)
Q_DECLARE_METATYPE(DicomDate)
Q_DECLARE_METATYPE(DicomDateRange)
Q_DECLARE_METATYPE(DicomTime)
Q_DECLARE_METATYPE(DicomTimeRange)
Q_DECLARE_METATYPE(DicomDateTime)
Q_DECLARE_METATYPE(DicomDateTimeRange)
Q_DECLARE_METATYPE(DicomFindIntRange<quint8>)
Q_DECLARE_METATYPE(DicomFindIntRange<quint16>)
Q_DECLARE_METATYPE(DicomFindIntRange<qint16>)
Q_DECLARE_METATYPE(DicomFindIntRange<quint32>)
Q_DECLARE_METATYPE(DicomFindIntRange<qint32>)
Q_DECLARE_METATYPE(DicomFindIntRange<quint64>)
Q_DECLARE_METATYPE(DicomFindIntRange<qint64>)
Q_DECLARE_METATYPE(DicomFindIntRange<double>)
Q_DECLARE_METATYPE(DicomFindIntRange<float>)

CAP_DICOMLIB_EXPORT QDebug operator<< (QDebug, const DicomTzOffset& value);
CAP_DICOMLIB_EXPORT QDebug operator<< (QDebug, const DicomDate& value);
CAP_DICOMLIB_EXPORT QDebug operator<< (QDebug, const DicomDateRange& value);
CAP_DICOMLIB_EXPORT QDebug operator<< (QDebug, const DicomTime& value);
CAP_DICOMLIB_EXPORT QDebug operator<< (QDebug, const DicomTimeRange& value);
CAP_DICOMLIB_EXPORT QDebug operator<< (QDebug, const DicomDateTime& value);
CAP_DICOMLIB_EXPORT QDebug operator<< (QDebug, const DicomDateTimeRange& value);
CAP_DICOMLIB_EXPORT QDebug operator<< (QDebug, const DicomFindIntRange<quint8>& value);
CAP_DICOMLIB_EXPORT QDebug operator<< (QDebug, const DicomFindIntRange<quint16>& value);
CAP_DICOMLIB_EXPORT QDebug operator<< (QDebug, const DicomFindIntRange<qint16>& value);
CAP_DICOMLIB_EXPORT QDebug operator<< (QDebug, const DicomFindIntRange<quint32>& value);
CAP_DICOMLIB_EXPORT QDebug operator<< (QDebug, const DicomFindIntRange<qint32>& value);
CAP_DICOMLIB_EXPORT QDebug operator<< (QDebug, const DicomFindIntRange<quint64>& value);
CAP_DICOMLIB_EXPORT QDebug operator<< (QDebug, const DicomFindIntRange<qint64>& value);
CAP_DICOMLIB_EXPORT QDebug operator<< (QDebug, const DicomFindIntRange<double>& value);
CAP_DICOMLIB_EXPORT QDebug operator<< (QDebug, const DicomFindIntRange<float>& value);
