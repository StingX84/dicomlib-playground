//////////////////////////////////////////////////////////////////////////////
/// \file dpxdicom/data/cdataset.h
/// \brief Файл описания класса #CDataset
/// \author Девятников А.В.
/// \date 2012-05-05
///
/// Copyright (C) 2025 by RTK Radiology
/// ALL RIGHTS RESERVED.
//////////////////////////////////////////////////////////////////////////////

#pragma once

#include "dpxdicom/dicomlib.h"

#include "dpxdicom/data/cdicomencoding.h"

#include <QDateTime>
#include <QVector>

#include <dcmtk/dcmdata/dcobject.h>
#include <dcmtk/dcmdata/dctag.h>
#include <dcmtk/dcmdata/dcxfer.h>

#include <string_view>

// Преодопределения из внешних библиотек
class QJsonValue;
class QJsonObject;

// Предопределения

// Преодопределения других типов DICOMLIB
class CDicomAssociation;
interface IDicomConfiguration;
class CDatasetSequence;

// Предопределения классов в этом файле
class CDataset;

#define DCM_EXTRA_FLAG		"    "
#define DCM_EXTRA_FLAG_SIZE 4

/** Тип карты от тэга к значению атрибута.
 *
 * Эта карта предназначена для передачи данных датасета через механизм QVariant.
 *
 * В случае обработки C-FIND-RQ датасетов или атрибутов, пришедших или отправляемых в БД, необходимо использовать тип
 * данных `DicomAttributes`, который обрабатывается классом #CDicomAttributesMapper.
 *
 * \sa CDataset::putAttributes, CDataset::getAttributesAsMap
 */
typedef QMap<DcmTag, QVariant> DatasetVariantMap;

/** Тип датасета с общим типом владения */
typedef QSharedPointer<CDataset> CDatasetPtr;

/** Класс для работы с датасетами
 *
 * Представляет собой адаптер над датасетом DCMTK (DcmItem), предоставляющий следующие возможности для приложения:
 * - Автоматическая обработка частных атрибутов (см. #Config::ResolvePrivateTags):
 *   - Поиск реального тэга частного атрибута при любом чтении или записи значений.
 *   - Запись атрибута Private Reservation при записи частных атрибутов.
 *   - Авто-удаление соответствующего Private Reservation при удалении частного атрибута.
 *   - Конвертация частных атрибутов датасета в их "справочный" вид при чтении атрибутов в виде #DicomVariantMap или
 *     QJsonObject.
 * - Автоматический учет атрибута Timezone Offset from UTC (0008,0201):
 *   - При записи и чтении VR = DT (QDateTime, QVariant, QJsonValue)
 *   - При записи и чтении комбинированного значения QDate / QTime в разных тэгах.
 * - Автоматический учет атрибута Specific Character Set (0008,0005):
 *   - Учитывается при чтении атрибутов, на которые распространяется кодировка.
 *   - Автоматически записывается в датасет по необходимости.
 * - Преобразование в/из типов Qt (QDate, QTime, QDateTime, QVariant, QJsonValue, QString, QByteArray, QVector, ...)
 *
 * Семантика копирования класса: Value: класс нельзя копировать, а можно только перемещать.
 * Для того, чтобы передавать и хранить датасет рекомендуется использовать #CDatasetPtr.
 *
 * Все методы класса являются reentrant.
 *
 * Низлежащий DCMTK объект может быть одним из трех типов ( #type() ):
 * - #Type::Dataset - Обычный датасет (`DcmDataset`).
 * - #Type::MetaInfo - Датасет, содержащий заголовок DICOM файла (`DcmMetaInfo`).
 * - #Type::Item - Элемент атрибута с типом SQ (последовательность) (`DcmItem`).
 *
 * Владение объектом DCMTK ( #dcm() ) осуществляется одним из двух способов ( см. #ownsDcm() ):
 * - Захват - Объект CDataset "владеет" указателем на DCMTK объект и удалит его при удалении своей последней копии.
 * - Ссылка - Объект CDataset ссылается на DCMTK объект и не будет его удалять.
 *
 * Тип владения можно указать при создании объекта класса, принимающим аргумент типа #Capture.
 *
 * Датасет может быть элементом сиквенса. Причем, DCMTK не запрещает добавлять в качестве элемента и обычные датасеты
 * и объекты описания мета информации о файле.
 */
class CAP_DICOMLIB_EXPORT CDataset
{
public:
	/** Тип внутренних данных датасета */
	enum class Type
	{
		Dataset,  ///< Внутренними данными является датасет
		MetaInfo, ///< Внутренними данными является метазаголовок
		Item,	  ///< Внутренними данными является элемент последовательности
	};

	/** Тип захвата переданного DCMTK указателя */
	enum class Capture
	{
		Own,	   ///< Захватить владение указателем.
		Reference, ///< Ссылаться на указатель, но не владеть им. Указатель должен жить все время, пока жив CDataset
		Clone,	   ///< Скопировать датасет и завладеть копией.
	};

	/// Флаги для чтения и записи значений атрибутов.
	enum class Flags
	{
		/** Признак строгого соответствия стандарту DICOM во всех операциях датасета.
		 *
		 * Если флаг не установлен, то:
		 * - При ошибке чтения офсета времени в атрибуте DT будет возвращен нулевой офсет.
		 * - В числовых значениях допускается запятая вместо точки.
		 * - В целочисленных числовых значениях допускаются дробные числа.
		 * - При вырезании пробелов используются не только указанный в стандарте SPACE, но и символы '\\t', '\\f',
		 *   '\\v','\\r', '\\n', '\\0'
		 * - Допускается неточное преобразование между различными числовыми типами.
		 *
		 * Также, некоторые части программы основываются на этом флаге во время работы для "обхода" ошибок разбора
		 * данных или возврата ошибки при разборе.
		 *
		 * По умолчанию не установлен
		 */
		StrictMode = 1 << 0,

		/** Признак обработки запроса QueryRetrieve (C-FIND-RQ, C-MOVE-RQ и C-GET-RQ)
		 *
		 * Этот признак в самом датасете влияет на поведение в обработке значений атрибутов типов DA, DT и TM.
		 *
		 * По умолчанию: не установлен
		 */
		IsQueryRetrieve = 1 << 1,

		/** Признак форсирования записи временной зоны для типов DT
		 *
		 * Если этот признак установлен, то при формировании текста из типов QDateTime, DicomDateTime,
		 * DicomDateTimeRange всегда будет записан офсет от UTC.
		 *
		 * По умолчанию: не установлен
		 */
		AlwaysWriteTzOffset = 1 << 2, ///< Всегда записывать DT с офсетом времени

		/** Признак автоматической учета и обработки частных атрибутов датасетом.
		 *
		 * При включении этого флага, включаются следующие алгоритмы:
		 * 1. Поиск реального значения частного атрибута вызовом #resolvePrivateTag при любой попытке чтения атрибутов.
		 * 2. Поиск и добавление резервации вызовом #reservePrivateTag при любой попытке записи атрибутов.
		 * 3. Удаление резерваций после удаления всех атрибутов из них.
		 *
		 * По умолчанию: установлен
		 */
		ResolvePrivateTags = 1 << 3,

		/** Признак задействования особой обработки типов DA, DT и TM при извлечении их в QVariant или в QJsonValue
		 *
		 * Если флаг не задан, то все эти VR возвращаются в виде QByteArray или QVector<QByteArray> без изменений.
		 *
		 * Если флаг задан, то в результате могут вернуться значения QDate, QTime, QDateTime, QPair от них, QByteArray.
		 * Если значений несколько, то QVector<>
		 *
		 * Каждое значение вычисляется следующим образом:
		 * 1. Если значение пусто, то невалидный QVariant
		 * 2. Если значением являются две двойные кавычки ('""') то `QByteArray` с двумя двойными кавычками.
		 * 3. Если значением является звездочка ('*'), флаг #IsQueryRetrieve установлен, а флаг #StrictMode нет, то
		 * невалидный QVariant.
		 * 4. В зависимости от `VR` и флага #IsQueryRetrieve:
		 *    - DA c #IsQueryRetrieve:
		 *      - `QDate`, если текст не содержит "-"
		 *      - `QPair<QDate, QDate>`, если текст содержит "-"
		 *    - DA без #IsQueryRetrieve:
		 *      - `QDate`
		 *    - DT с #IsQueryRetrieve:
		 *      - `QDateTime`, если текст не содержит "-" И все компоненты даты и времени заданы, включая миллисекунды.
		 *      - `QPair<QDateTime, QDateTime>` в противном случае
		 *    - DT без #IsQueryRetrieve:
		 *      - `QDateTime`
		 *    - TM с #IsQueryRetrieve:
		 *      - `QTime`, если текст не содержит "-" И миллисекунды заданы.
		 *      - `QPair<QTime, QTime>` в противном случае
		 *    - TM без #IsQueryRetrieve:
		 *      - `QTime`
		 * 5. Если текст не удалось разобрать, то возвращается ошибка.
		 *
		 * При записи QJsonValue используется тот же алгоритм с превращением Qt типов следующим образом:
		 * - `array` если записывается `QVector<QVariant>`
		 * - `null`, для невалидного `QVariant`
		 * - `object` с двумя ключами `from` и `to` для `QPair<..., ...>`
		 * - `string` с ISO 8601 форматом записи для `QDate`, `QTime` и `QDateTime`:
		 *   - **YYYY-MM-DD**
		 *   - **HH:mm:ss.SSS**
		 *   - **YYYY-MM-DDTHH:mm:ss.SSS±HH:mm** Временная зона указывается всегда, даже если ее не было в атрибуте
		 *     и в самом датасете она не установлена.
		 *
		 * По умолчанию: установлен
		 */
		ParseQtDates = 1 << 4,

		/** Признак задействования особой обработки типов DA, DT и TM при извлечении их в QVariant
		 *
		 * Если флаг не задан, то все эти VR возвращаются в виде QByteArray или QVector<QByteArray> без изменений.
		 *
		 * Если задан флаг #ParseQtDates, то этот флаг игнорируется и обработка ведется по правилам #ParseQtDates.
		 *
		 * Если флаг задан, то в результате могут вернуться значения #DicomDate, #DicomTime, #DicomDateTime,
		 * #DicomDateRange, #DicomTimeRange, #DicomDateTimeRange, QByteArray.
		 * Если значений несколько, то QVector<QVariant>, где каждый QVariant - это отдельное значение с типом,
		 * приведенным выше.
		 *
		 * Каждое значение вычисляется следующим образом:
		 * 1. Если пусто, то невалидный QVariant
		 * 2. Если значением являются две двойные кавычки ('""') то `QByteArray` с двумя двойными кавычками.
		 * 3. Если значением является звездочка ('*'), флаг #IsQueryRetrieve, а флаг #StrictMode нет, то невалидный
		 * QVariant.
		 * 4. В зависимости от `VR` и флага #IsQueryRetrieve:
		 *    - DA c #IsQueryRetrieve:
		 *      - #DicomDate, если текст не содержит "-"
		 *      - #DicomDateRange, если текст содержит "-"
		 *    - DA без #IsQueryRetrieve:
		 *      - #DicomDate
		 *    - DT с #IsQueryRetrieve:
		 *      - #DicomDateTime, если текст не содержит "-".
		 *      - #DicomDateTimeRange в противном случае
		 *    - DT без #IsQueryRetrieve:
		 *      - #DicomDateTime
		 *    - TM с #IsQueryRetrieve:
		 *      - #DicomTime, если текст не содержит "-".
		 *      - #DicomTimeRange в противном случае
		 *    - TM без #IsQueryRetrieve:
		 *      - #DicomTime
		 * 5. Если текст не удалось разобрать, то возвращается ошибка.
		 *
		 * По умолчанию: не установлен
		 */
		ParseDicomDates = 1 << 5,

		/** Признак исключения сервисных атрибутов при создании #DatasetVariantMap и `QJsonValue`.
		 *
		 * Список сервисных атрибутов:
		 * - Все **Private Reservation** (gggg,00xx), где gggg - нечетная.
		 * - Все длины групп (gggg,0000)
		 * - Timezone Offset from UTC (0008,0201)
		 * - Specific Character Set (0008,0005)
		 *
		 * По умолчанию: установлен
		 */
		SkipServiceTags = 1 << 6,
	};

	/** Символ разделитель нескольких значений в атрибуте */
	template<class T> static constexpr const T VALUES_DELIMITER = T('\\');

	/** Константа с флагами по умолчанию */
	static constexpr const int DEFAULT_FLAGS
		= (int)Flags::ParseQtDates | (int)Flags::ResolvePrivateTags | (int)Flags::SkipServiceTags;

public: // Методы инициализации

	/** Конструктор, создающий "пустой" датасет.
	 *
	 * Создает пустой нулевой датасет (#isNull() == true, #isEmpty() == true).
	 */
	explicit CDataset(Type type = Type::Dataset, const CDataset* parent = nullptr) noexcept;

	/** Конструктор от существующего объекта DCMTK
	 *
	 * Внутри вызывается #syncEncodingAndTzOffset() для обновления кэша офсета времени и кодировки.
	 *
	 * \param dcm Объект DCMTK. Допускаются объекты классов `DcmItem`, `DcmDataset` и `DcmMetaInfo`.
	 * \param capture Режим захвата объекта \a dcm.
	 * \param parent Опциональный указатель на родительский или корневой датасет.
	 */
	explicit CDataset(DcmItem* dcm, Capture capture = Capture::Own, const CDataset* parent = nullptr) noexcept;

	CDataset(const CDataset&) noexcept = delete;
	CDataset(CDataset&&) noexcept;
	CDataset& operator= (const CDataset&) noexcept = delete;
	CDataset& operator= (CDataset&&) noexcept;

	~CDataset() noexcept;

	void swap(CDataset&) noexcept;

	/** Клонирует текущий датасет.
	 *
	 * Этот метод создает новый объект DCMTK глубоким копированием текущего объекта.
	 *
	 * Возвращенный объект:
	 * - имеет настройки от текущего корневого датасета.
	 * - Является корневым сам для себя.
	 * - По необходимости, в него добавляются атрибуты Specific Character Set (0008,0005) и
	 *   Timezone Offset from UTC (0008,0201)
	 */
	CDataset clone() const noexcept;

	/** Создает внутренний DCMTK объект, если он не был создан ранее (#isNull == \c true)
	 *
	 * Метод вызывается автоматически во всех методах изменяющих данные.
	 * Ручной вызов может быть полезен перед использованием объекта из #dcm
	 */
	void init() noexcept;

	/** Освобождает старый указатель на датасет и устанавливает новый \a newDcm.
	 *
	 * Все настройки датасета и принадлежность к корневому не изменяются.
	 *
	 * Метод вызывает #syncEncodingAndTzOffset() для обновления кэша офсета времени и кодировки.
	 *
	 * \param newDcm (опционально) Новый обернутый указатель DCMTK. Если указан, то тип датасета может измениться
	 * вслед за типом переданного объекта.
	 * \param capture (опционально) Режим захвата указателя \a newDcm.
	 */
	void reset(DcmItem* newDcm = nullptr, Capture capture = Capture::Own) noexcept;

	/** Обновляет внутренние переменные кодировки и офсета времени для соответствия данным датасета.
	 *
	 * Этот метод нужно вызывать только при изменении атрибутов средствами DCMTK. Операции, выполняемые посредством
	 * этой библиотеки автоматически поддерживают внутренний кэш.
	 *
	 * Атрибуты, после изменения которых желательно вызвать данный метод:
	 * - Timezone Offset from UTC (0008,0201)
	 * - Specific Character Set (0008,0005)
	 */
	void syncEncodingAndTzOffset() noexcept;

	/** Возвращает указатель на сохраненный внутри объект DCMTK и забывает о нем.
	 *
	 * Соблюдение времени жизни возвращенного объекта передается вызывающему.
	 */
	DcmItem* release() noexcept;

	/** Преобразует набор пар ключ - значение в датасет
	 *
	 * В случае ошибки преобразования хотя бы одного значения QVariant, возвращается нулевой датасет.
	 *
	 * см. #putAttributes(const std::initializer_list<std::pair<DcmTag, QVariant>>&)
	 *
	 * \param type Тип создаваемого датасета
	 * \param root Корневой датасет, настройки которого будут использованы. Если не указан, то новый датасет
	 * будет сам себе корневым. Этот параметр рекомендуется указывать, если предполагается вставлять датасет
	 * в качестве элемента сиквенса.
	 * \param flags Флаги #configFlags. Используются только если \a root == nullptr
	 */
	static CDataset fromAttributes(const std::initializer_list<std::pair<DcmTag, QVariant>>& values,
								   Type type = Type::Dataset, const CDataset* root = nullptr, int flags = DEFAULT_FLAGS);

	/** Преобразует набор пар ключ - значение в датасет
	 *
	 * В случае ошибки преобразования хотя бы одного значения QVariant, возвращается нулевой датасет.
	 *
	 * см. #putAttributes(const DatasetVariantMap&)
	 *
	 * \param type Тип создаваемого датасета
	 * \param root Корневой датасет, настройки которого будут использованы. Если не указан, то новый датасет
	 * будет сам себе корневым. Этот параметр рекомендуется указывать, если предполагается вставлять датасет
	 * в качестве элемента сиквенса.
	 * \param flags Флаги #configFlags. Используются только если \a root == nullptr
	 */
	static CDataset fromAttributes(const DatasetVariantMap& values, Type type = Type::Dataset, const CDataset* root = nullptr, int flags = DEFAULT_FLAGS);

	/** Преобразует QJsonObject в датасет
	 *
	 * В случае ошибки преобразования хотя бы одного значения QJsonValue или его ключа, возвращается нулевой датасет.
	 *
	 * см. #putAttributes(const QJsonObject&)
	 *
	 * \param type Тип создаваемого датасета
	 * \param root Корневой датасет, настройки которого будут использованы. Если не указан, то новый датасет
	 * будет сам себе корневым. Этот параметр рекомендуется указывать, если предполагается вставлять датасет
	 * в качестве элемента сиквенса.
	 * \param flags Флаги #configFlags. Используются только если \a root == nullptr
	 */
	static CDataset fromAttributes(const QJsonObject& object, Type type = Type::Dataset, const CDataset* root = nullptr, int flags = DEFAULT_FLAGS);

public: // Основная информация об объекте

	/** Возвращает признак отсутствия обернутого DCMTK объекта.
	 *
	 * Объект изначально нулевой в ситуациях:
	 * - Создан конструктором по умолчанию
	 * - Создан конструктором с передачей нулевого объекта DCMTK
	 * - Создан конструктором перемещения от другого нулевого объекта
	 *
	 * Объект становится нулевым в ситуациях:
	 * - Произошел вызов #release
	 * - Произошел вызов #reset с nullptr
	 * - Объект добавлен в #CDatasetSequence
	 * - Объект перемещен в другой объект оператором или конструктором перемещения
	 * - Объект свапнут с другим нулевым объектом
	 *
	 * Объект перестает быть нулевым в ситуациях:
	 * - Вызван любой метод, в котором может произойти модификация или запись DCMTK атрибута.
	 * - Вызван #init
	 * - Вызван #reset с передачей ненулевого указателя на объект DCMTK
	 * - В этот объект перемещен другой ненулевой объект оператором перемещения
	 * - Объект свапнут с другим ненулевым объектом
	 */
	inline bool isNull() const noexcept { return m_dcm != nullptr; }

	/** Возвращает признак того, что в датасете нет ни одного атрибута или указатель на DCMTK объект не установлен. */
	bool isEmpty() const noexcept;

	/** Возвращает тип оборачиваемого объекта DCMTK.
	 *
	 * Этот тип задается при конструировании объекта и не может быть изменен позднее.
	 */
	inline Type type() const noexcept { return m_type; }

	/** Возвращает содержащийся внутри указатель на DCMTK объект */
	inline DcmItem* dcm() const noexcept { return m_dcm; }

	/** Возвращает признак владения указателем #dcm()
	 *
	 * Если объект владеет этим указателем, то он сам удалит его в своем деструкторе.
	 */
	inline bool ownsDcm() const noexcept { return m_dcmOwned; }

	/** Возвращает указатель на корневой датасет. Может указывать сам на себя. */
	inline CDataset* root() const noexcept { return m_root; }

public: // Сравнение по каждому атрибуту

	/** Сравнивает датасеты по атрибутно */
	int compare(const CDataset& o) const noexcept;

	/** Оператор сравнения с другим датасетом по атрибутно */
	inline bool operator== (const CDataset& right) const noexcept { return compare(right) == 0; }

	/** Оператор сравнения с другим датасетом по атрибутно */
	inline bool operator< (const CDataset& right) const noexcept { return compare(right) < 0; }

	/** Оператор сравнения с другим датасетом по атрибутно */
	inline bool operator<= (const CDataset& right) const noexcept { return compare(right) <= 0; }

	/** Оператор сравнения с другим датасетом по атрибутно */
	inline bool operator!= (const CDataset& right) const noexcept { return !operator== (right); }

	/** Оператор сравнения с другим датасетом по атрибутно */
	inline bool operator> (const CDataset& right) const noexcept { return !operator<= (right); }

	/** Оператор сравнения с другим датасетом по атрибутно */
	inline bool operator>= (const CDataset& right) const noexcept { return !operator< (right); }


public: // Настройки конфигурации

	/** Устанавливает битовую маску флагов #Flags в \a flags, определяющую настройки работы класса */
	inline void configSetFlags(int flags) noexcept { m_root->m_config.flags = flags; }

	/** Возвращает битовую маску флагов #Flags, определяющую настройки работы класса */
	inline int configFlags() const noexcept { return m_root->m_config.flags; }

	/** Устанавливает флаг \a flag настройки работы класса */
	inline void configSetFlag(Flags flag) noexcept { m_root->m_config.flags |= int(flag); }

	/** Сбрасывает флаг \a flag настройки работы класса */
	inline void configClearFlag(Flags flag) noexcept { m_root->m_config.flags &= ~int(flag); }

	/** Устанавливает или сбрасывает флаг \a flag настройки работы класса */
	inline void configSetFlag(Flags flag, bool set) noexcept
	{
		if (set)
			configSetFlag(flag);
		else
			configClearFlag(flag);
	}

	/** Возвращает признак установки флага \a flag в флагах #configFlags */
	inline bool configHasFlag(Flags flag) const noexcept { return (m_root->m_config.flags & int(flag)) != 0; }

	/** Возвращает символ разделитель, используемый для разделения значений атрибутов с `VR` == `PN`.
	 *
	 * По умолчанию: BACKSLASH "\\" (стандартный разделитель DICOM).
	 *
	 * \sa configUpdateFromAssociation, configUpdateFromConfiguration, configSetPnDelimiter
	 */
	inline QChar configPnDelimiter() const noexcept { return m_root->m_config.pnDelimiter; }

	/** Устанавливает символ разделитель для разделения значений атрибутов с `VR` == `PN`.
	 * \sa configUpdateFromAssociation, configUpdateFromConfiguration, configPnDelimiter
	 */
	inline void configSetPnDelimiter(QChar val) noexcept { m_root->m_config.pnDelimiter = val; }

	/** Извлекает настройки обработки датасетов из конфигурации, установленное в ассоциации \a assoc.
	 * \param assoc Указатель на ассоциацию. Если \c nullptr, то метод ничего не делает.
	 * \sa configUpdateFromConfiguration
	 */
	void configUpdateFromAssociation(const CDicomAssociation* assoc);

	/** Извлекает настройки обработки датасетов из конфигурации \a config с параметрами подключения \a connection
	 *
	 * Извлекаемые параметры:
	 * - #IDicomConfiguration::AeStrictMode - обновляет флаг #Flags::StrictMode в #configFlags
	 * - #IDicomConfiguration::AlwaysWriteTimeOffset - обновляет флаг #Flags::AlwaysWriteTzOffset в #configFlags
	 * - #IDicomConfiguration::DefaultEncoding - записывается в кэш, если в датасете еще не установлена кодировка.
	 * - #IDicomConfiguration::PNDelimiter - обновляет #configPnDelimiter
	 *
	 * Кодировка обрабатывается особым образом: она запоминается в кэше без признака того, что есть
	 * в датасете. В дальнейшем метод #encoding возвращает именно ее, а при записи любого атрибута, на
	 * который распространяется действие кодировки она реально будет записана в датасет.
	 *
	 * \param config Указатель на объект конфигурации. Если \c nullptr, то метод ничего не делает.
	 * \param connection Указатель на параметры соединения. Если \a nullptr, то метод извлекает настройки без
	 * учета соединения.
	 *
	 * \sa configUpdateFromAssociation
	 */
	void configUpdateFromConfiguration(const IDicomConfiguration* config, DicomConnectionBase* connection);


public: // Кодировка текста

	/** Возвращает текущую кодировку датасета.
	 *
	 * Текущая кодировка хранится в атрибуте Specific Character Set (0008,0005) и извлекается в кэш
	 * в конструкторе класса, а также при его изменении через методы этого класса.
	 *
	 * Если атрибут изменяется другими методами, то после такого изменения необходимо вызвать
	 * #syncEncodingAndTzOffset().
	 *
	 * Метод можно вызывать для элементов сиквенса, но он все равно вернет значение из корневого класса.
	 * Замечание: если датасет помещен в сиквенс, но у сиквенса не определен #CDicomSequence::root, то
	 * этот датасет считает себя корневым. Это нормальная ситуация, если сиквенс сначала создать без
	 * указания датасета назначения, наполнить его элементами, а, затем, добавить в некоторый датасет.
	 *
	 * Кодировка автоматически адаптируется при добавлении датасета в сиквенс в другом датасете. В этом
	 * случае, уже существующий атрибут Specific Character Set (0008,0005) будет удален, а все атрибуты, на
	 * которые распространяется кодировка перезаписываются. (см. #changeEncoding и закрытый метод #changeRoot).
	 *
	 * \return
	 * - Значение атрибута Specific Character Set (0008,0005)
	 * - Значение #CDicomEncoding::sysDefault
	 */
	const CDicomEncoding& encoding() const;

	/** Устанавливает текущую кодировку датасета.
	 *
	 * См. #encoding для детального описания работы с кодировкой.
	 *
	 * Действия над атрибутом Specific Character Set (0008,0005):
	 * - Если передан \a forceWriteAttribute, то атрибут будет записан безусловно. Если \a encoding пустой,
	 *   то будет записан #CDicomEncoding::sysDefault.
	 * - Если \a encoding пустой (создан конструктором по умолчанию), то атрибут будет удален.
	 * - Если \a encoding не пустой, то атрибут будет записан если тип датасета != Type::MetaInfo. Если он
	 *   уже есть в датасете, то будет обновлен вне зависимости от типа датасета.
	 *
	 * Система использует для записи актуальный термин #CDicomEncoding::term в \a encoding, даже если он не валидный.
	 *
	 * Атрибут записывается только в корневой датасет. Вызов этого метода в элементе сиквенса вызовет ASSERT.
	 *
	 * Уже существующие атрибуты на которые распространяется кодировка в датасете не изменяются.
	 * Для изменения кодировки с изменением существующих атрибутов, используется метод
	 * #changeEncoding.
	 */
	CAPRESULT setEncoding(const CDicomEncoding& encoding, bool forceWriteAttribute = false);

	/** Производит перекодировку всех переводимых атрибутов датасета без учета текущей кодировки датасета.
	 *
	 * Этот метод производит перекодировку только внутри данного датасета и дочерних элементов сиквенса.
	 *
	 * \param fromEncoding  Кодировка с которой производится чтение атрибутов.
	 * \param toEncoding Кодировка с которой производится запись атрибутов.
	 * \return
	 * - capTrue - В датасет внесены изменения (имеются атрибуты, на которые кодировка повлияла)
	 * - capFalse - В датасет ни одного изменения не внесено.
	 * - Другой код в случае ошибок.
	 */
	CAPRESULT changeEncoding(const CDicomEncoding& fromEncoding, const CDicomEncoding& toEncoding,
							 bool updateEncoding = false);

public: // Смещение времени в секундах от UTC для датасета

	// Константа с значением смещения времени равным текущему системному.
	static constexpr qint32 LOCAL_TIME_OFFSET = std::numeric_limits<qint32>::max();

	/** Возвращает текущее смещение времени от UTC для датасета в сеукндах.
	 *
	 * Текущее значение хранится в атрибуте Timezone Offset from UTC (0008,0201) и извлекается в кэш
	 * в конструкторе класса, а также при его изменении через методы этого класса.
	 *
	 * Если атрибут изменяется другими методами, то после такого изменения необходимо вызвать
	 * #syncEncodingAndTzOffset().
	 *
	 * Метод можно вызывать для элементов сиквенса, но он все равно вернет значение из корневого класса.
	 * Замечание: если датасет помещен в сиквенс, но у сиквенса не определен #CDicomSequence::root, то
	 * этот датасет считает себя корневым. Это нормальная ситуация, если сиквенс сначала создать без
	 * указания датасета назначения, наполнить его элементами, а, затем, добавить в некоторый датасет.
	 *
	 * Временная зона автоматически адаптируется при добавлении датасета в сиквенс в другом датасете. В этом
	 * случае, уже существующий атрибут Timezone Offset from UTC (0008,0201) будет удален, а все DT атрибуты
	 * и DA+TM пары будут пересчитаны под изменившееся смещение (см. #changeTimezoneOffsetFromUtc и
	 * закрытый метод #changeRoot).
	 *
	 * \param systemIfNotFound Флаг инструктирующий метод на возврат текущей локального смещения времени
	 * вместо #LOCAL_TIME_OFFSET.
	 * \param[out] rvExtractedFromDatasdet Опциональный флаг, в котором возвращается признак наличия атрибута
	 * в датасете. Этот флаг кэширован и реальный поиск атрибута в датасете в этом методе не происходит!
	 *
	 * \return
	 * - Значение атрибута Timezone Offset from UTC (0008,0201)
	 * - Значение #LOCAL_TIME_OFFSET (атрибута в датасете нет или его невозможно разобрать)
	 */
	qint32 timezoneOffsetFromUtc(bool systemIfNotFound = false, bool* rvExtractedFromDatasdet = nullptr) const;

	/** Устанавливает текущее смещение времени от UTC для датасета в сеукндах.
	 *
	 * См. #timezoneOffsetFromUTC для детального описания работы со смещением.
	 *
	 * Действия над атрибутом Timezone Offset from UTC (0008,0201):
	 * - Если передан forceWriteAttribute, то атрибут будет записан безусловно. Даже если value == #LOCAL_TIME_OFFSET.
	 * - Если value == #LOCAL_TIME_OFFSET, то атрибут будет удален.
	 * - Если value != #LOCAL_TIME_OFFSET, то атрибут будет записан если тип датасета != Type::MetaInfo. Если он
	 *   уже есть в датасете, то будет обновлен вне зависимости от типа датасета.
	 *
	 * Атрибут записывается только в корневой датасет. Вызов этого метода в элементе сиквенса вызовет ASSERT.
	 *
	 * Уже существующие DT и DA+TM атрибуты в датасете не изменяются.
	 * Для изменения временной зоны с пересчетом существующих атрибутов, используется метод
	 * #changeTimezoneOffsetFromUtc.
	 */
	CAPRESULT setTimezoneOffsetFromUtc(qint32 value, bool forceWriteAttribute = false);

	/** Перерасчитывает и обновляет все DT и парные DA+TM атрибуты в датасете и дочерних сиквенсах.
	 *
	 * Изменению подвергаются:
	 * - Все DT атрибуты, у которых не указано смещение
	 * - Все пары из DA+TM (см. #CDicomTagInfo::daTmPair).
	 *
	 * Поддерживает также специфические для C-FIND диапазоны значений "Range Matching".
	 *
	 * Если значение атрибута DA, DT или TM не смогло быть распознано, то атрибут молча игнорируется.
	 *
	 * Опция #Config::AlwaysWriteTzOffset не учитывается в этом методе, т.к. он максимально старается сохранить
	 * точность и форму записи исходного атрибута.
	 *
	 * \param fromTzOffset Офсет от UTC в секундах с которым записаны атрибуты на текущий момент.
	 * \param toTzOffset Офсет от UTC в секундах в котором атрибуты должны быть записаны.
	 * \param writeTimezoneOffsetFromUTC Включает вызов #setTimezoneOffsetFromUTC после выполнения конвертации.
	 */
	CAPRESULT changeTimezoneOffsetFromUtc(qint32 fromTzOffset, qint32 toTzOffset,
										  bool writeTimezoneOffsetFromUTC = false);

public: // Методы загрузки и сохранения файла

	/** Загружает датасет из файла на диске
	 *
	 * Замечание: Этот метод может произвести чтение датасета с обходом некоторых проблем. В случае, если такой
	 * обход был осуществлен, возвращается флаг \a rvRequireRebuild. Это флаг уведомляет о том, что стандартные
	 * функции DCMTK для чтения датасета не сработают. Флаг используется в ситуациях, когда приложение планирует
	 * использовать файл в дальнейшем и "пересохраняет" файл при получении такого флага.
	 *
	 * \param fileName Имя файла
	 * \param[out] rvDataset Возвращаемый объект датасета. Может быть nullptr, если не требуется.
	 * \param[out] rvMetaInfo Возвращаемый объект метаинформации о датасете. Может быть nullptr, если не требуется.
	 * \param readXfer Синтаксис передачи датасета на диске. Если не указано, то автоматически определяется из заголовка
	 * или простым анализом первых байт файла.
	 * \param groupLength Тип обработки атрибутов с длинами групп. Это пережиток старых времен и в текущем стандарте
	 * длины групп не используются, но для совместимости параметр по умолчанию оставляет их в датасете, если они
	 * существуют в файле.
	 * \param maxReadLength Максимальная длина значения атрибута более которой он не загружается немедленно, а в
	 * режиме "ленивой" загрузки при первом использовании или при вызове #loadAllDataIntoMemory.
	 * \param stopParsingAtElement Тэг атрибута после которого необходимо прекратить чтение файла.
	 * \param[out] rvRequireRebuild Возвращаемый параметр с флагом, устанавливаемым в true, если файл "битый" и для
	 * дальнейшей нормальной работы необходимо его пересохранение.
	 * \return Код ошибки
	 */
	static CAPRESULT loadFromFile(const QString& fileName, CDataset* rvDataset, CDataset* rvMetaInfo = nullptr,
								  const E_TransferSyntax readXfer = EXS_Unknown,
								  const E_GrpLenEncoding groupLength = EGL_noChange,
								  const Uint32 maxReadLength = DCM_MaxReadLength,
								  const DcmTagKey& stopParsingAtElement = DCM_UndefinedTagKey,
								  bool* rvRequireRebuid = nullptr);

	/** Загружает датасет из памяти
	 * \param data Указатель на байтовый массив, откуда производится чтение.
	 * \param dataLength Длина байтового массива в \a data.
	 * \param[out] rvDataset Возвращаемый объект датасета. Может быть nullptr, если не требуется.
	 * \param[out] rvMetaInfo Возвращаемый объект метаинформации о датасете. Может быть nullptr, если не требуется.
	 * \param readXfer Синтаксис передачи датасета. Если не указано, то автоматически определяется из заголовка
	 * или простым анализом первых байт.
	 * \param stopParsingAtElement Тэг атрибута после которого необходимо прекратить чтение файла.
	 * \return Код ошибки
	 */
	static CAPRESULT loadFromMemory(std::string_view mem, CDataset* rvDataset, CDataset* rvMetaInfo = nullptr,
									const E_TransferSyntax readXfer = EXS_Unknown,
									const DcmTagKey& stopParsingAtElement = DCM_UndefinedTagKey);

	/** Загружает из файла только заголовок
	 * \param fileName Имя файла
	 * \param[out] rvMetaInfo Возвращаемый объект метаинформации о датасете. Может быть nullptr, если не требуется.
	 * \param[out] rvIsCustomHeader (опционально) Возвращаемый флаг, уведомляющий о нестандартном заголовке файла.
	 * \return Код ошибки
	 */
	static CAPRESULT loadMetaFromFile(const QString& fileName, CDataset* rvMetaInfo = nullptr,
									  bool* rvIsCustomHeader = nullptr);


	/** Загружает все данные файла в память.
	 *
	 * Используется после вызова #loadFromFile, чтобы прочитать все пропущенные атрибуты и "отвзяаться" от файла.
	 */
	CAPRESULT loadAllDataIntoMemory();

	/** Производит сохранение датасета в файл
	 *
	 * Если файл уже существует, он будет перезаписан
	 *
	 * Этот вызов можно производить только если #type() == #Type::Dataset
	 *
	 * \param fileName Имя файла
	 * \param metaInfo Датасет, содержащий метаинформацию о текущем датасете. Должен быть с типом #Type::MetaInfo.
	 * \param customHeader Записывать нестандартный заголовок
	 * \param writeXfer Синтаксис передачи, в котором будет записан датасет
	 * \param encodingType Тип кодирования длинн сиквенсов и длинн групп.
	 * \param groupLength Тип пересчета атрибутов - длин групп.
	 * \param padEncoding Тип поддержки атрибутов "пэддинга"
	 * \param padLength Размер атрибутов пэддинга для датасета
	 * \param subPadLength Размер атрибутов пэддинга для элементов сиквенсов
	 * \return capOk в случае успеха или другой код в случае ошибки
	 */
	CAPRESULT saveToFile(const QString& fileName, const CDataset& metaInfo, bool customHeader = false,
						 E_TransferSyntax writeXfer = EXS_Unknown, E_EncodingType encodingType = EET_UndefinedLength,
						 E_GrpLenEncoding groupLength = EGL_recalcGL, E_PaddingEncoding padEncoding = EPD_noChange,
						 quint32 padLength = 0, quint32 subPadLength = 0) const;

	/** Сохраняет датасет в файл.
	 *
	 * Если файл уже существует, он будет перезаписан
	 *
	 * Этот вызов можно производить только если #type() == #Type::Dataset
	 *
	 * \param fileName Имя файла
	 * \param withMetaInfo Генерировать и сохранять метаинформацию о файле. См. #fillMetaInfo.
	 * \param customHeader Записывать нестандартный заголовок.
	 * \param writeXfer Синтаксис передачи, в котором будет записан датасет.
	 * \param encodingType Тип кодирования длинн сиквенсов и длинн групп.
	 * \param groupLength Тип пересчета атрибутов - длин групп.
	 * \param padEncoding Тип поддержки атрибутов "пэддинга"
	 * \param padLength Размер атрибутов пэддинга для датасета
	 * \param subPadLength Размер атрибутов пэддинга для элементов сиквенсов
	 * \return capOk в случае успеха или другой код в случае ошибки
	 */
	CAPRESULT saveToFile(const QString& fileName, bool writeMetaInfo = true, bool customHeader = false,
						 E_TransferSyntax writeXfer = EXS_Unknown, E_EncodingType encodingType = EET_UndefinedLength,
						 E_GrpLenEncoding groupLength = EGL_recalcGL, E_PaddingEncoding padEncoding = EPD_noChange,
						 quint32 padLength = 0, quint32 subPadLength = 0) const;

	/** Производит сохранение заголовка датасета в файл
	 *
	 * Если файл уже существует, он будет перезаписан
	 *
	 * Этот вызов можно производить только если #type() == #Type::MetaInfo
	 *
	 * \param fileName Имя файла
	 * \param customHeader Записывать нестандартный заголовок
	 * \return capOk в случае успеха или другой код в случае ошибки
	 */
	CAPRESULT saveMetaToFile(const QString& fileName, bool customHeader = false) const;

public: // Частные атрибуты

	/** Производит поиск в датасете резервации для группы частного атрибута \a tagKey и возвращает в \a rvTagKey
	 * исправленный номер элемента, соответствующий обнаруженной резервации.
	 *
	 * Результирующий \a rvTagKey определяется следующим образом:
	 * - Если \a tagKey не является частным, то он возвращается без изменений.
	 * - Если Private Creator не удалось определить (\a szPrivateCreator не задан и не установлен в справочнике), то:
	 *   - Если у \a tagKey старшие 8 бит номера элемента >= 0x10, то возвращается без изменений.
	 *   - В противном случае, возвращается ошибка (#capInvalidArg).
	 * - Если Private Creator удалось определить, то:
	 *   - Если обнаружен соответствующий Private Reservation, то:
	 *     Возвращается тэг с номером группы и младшими 8-ми битами номера элемента из \a tagKey, старшие 8 бит
	 *     номера элемента извлекаются из младших 8-ми бит Private Reservation.
	 *   - Если соответствующий Private Reservation не обнаружен, то:
	 *     - Если у \a tagKey старшие 8 бит номера элемента < 0x10, то возвращается ошибка (#capElementNotFound).
	 *     - Если для \a tagKey в месте "по умолчанию" уже существует некоторый Private Reservation, то возвращается
	 *       ошибка (#capElementNotFound).
	 *     - Если в таком месте "по умолчанию" нет Private Reservation, то возвращается \a tagKey без изменений.
	 *
	 * \param tagKey Искомый тэг.
	 * \param[out] rvTagKey Возвращаемый тэг.
	 * \param szPrivateCreator Значение `Private Creator`. Если не указано, то производится поиск в справочнике.
	 * \return
	 * - #capOk В случае, если в \a tagKey успешно возвращен.
	 * - #capInvalidArg В случае, если не удалось определить название резервации и атрибут \a tagKey не имеет номера
	 *   "по умолчанию" для резервации.
	 * - #capElementNotFound В случаях, если резервация не обнаружена и:
	 *   - Номера "по умолчанию" нет или;
	 *   - Номер резервации "по умолчанию" занят другим `Private Creator`.
	 */
	CAPRESULT
	resolvePrivateTag(const DcmTagKey& tagKey, DcmTagKey& rvTagKey, const char* szPrivateCreator = nullptr) const;

	/** Возвращает реальный тэг для указанного частного атрибута в датасете для целей чтения.
	 *
	 * Если определение завершилось ошибкой, то возвращается значение `DCM_UndefinedTagKey`.
	 *
	 * \sa resolvePrivateTag(const DcmTagKey&, DcmTagKey&, const char*)
	 */
	DcmTagKey resolvePrivateTag(const DcmTagKey& tagKey, const char* szPrivateCreator = nullptr) const;

	/** Производит поиск или добавляет в датасет резервацию для группы частного атрибута \a tagKey и возвращает
	 *  идентификатор атрибута с обновленным номером элемента в \a rvTagKey.
	 *
	 * Алгоритм поиска аналогичен #resolvePrivateTag(const DcmTagKey&, DcmTagKey&, const char*) за исключением
	 * того, что в случае отсутствия Private Reservation, он создается в датасете. Причем, если у частного
	 * атрибута заданы старшие 8 бит в номере элемента >= 0x10, то Private Reservation, по возможности, создается именно
	 * с этим числом в младших 8-ми битах. Если Private Reservation с таким тэгом уже существует, то происходит поиск
	 * первого свободного по порядку в диапазоне 0x10 .. 0xFF.
	 *
	 * \param tag Искомый тэг.
	 * \param[out] rvTag Возвращаемый тэг.
	 * \param szPrivateCreator Значение `Private Creator`. Если не указано, то производится поиск в справочнике.
	 * \return
	 * - #capOk В случае, если в \a tagKey возвращен исправленный атрибут или \a tagKey  без изменений.
	 * - #capLimitExceeded В случае, если в датасете не осталось свободных резерваций для группы частного атрибута.
	 * - #capInvalidArg В случае, если не удалось определить название резервации и атрибут \a tagKey не имеет
	 *   номера "по умолчанию" для резервации.
	 * - Другой код в случае ошибки создания элемента резервации в датасете.
	 */
	CAPRESULT reservePrivateTag(const DcmTag& tag, DcmTag& rvTag, const char* szPrivateCreator = nullptr);

	/** Возвращает реальный тэг для указанного частного атрибута в датасете для целей записи.
	 *
	 * Эта функция создает `Private Reservation` при необходимости.
	 *
	 * Может вернуть `DCM_UndefinedTagKey`, если запись с данным тэгом невозможна. Такое может произойти, если
	 * в датасете не осталось свободных резерваций для группы частного атрибута.
	 *
	 * \sa reservePrivateTag(const DcmTag&, DcmTag&, const char*)
	 */
	DcmTag reservePrivateTag(const DcmTag& tag, const char* szPrivateCreator = nullptr);

	/** Производит поиск тэга частного атрибута, полученного из этого датасета в справочнике тэгов и возращает
	 * обнаруженный тэг.
	 *
	 * Этот метод автоматически используется внутри датасета при создании объектов #DatasetVariantMap и `QJsonObject`
	 * в случае, если установлен флаг #Flags::ResolvePrivateTags.
	 *
	 * Использование данного метода позволяет приложению не учитывать тот факт, что частный атрибут в датасете
	 * может "численно" (номер группы + номер элемента) не равняться тэгу в константах приложения.
	 * Например, атрибут PR_TAG_PATIENT_UID может быть записан в датасете с номером (0009,6620). И если он именно
	 * с таким номером попадет в #DatasetVariantMap, то приложение уже не сможет его легко получить из такой карты
	 * используя константу PR_TAG_PATIENT_UID.
	 *
	 * \param tag Тэг атрибута из этого датасета.
	 * \return
	 * - Исходный \a tag без изменений, если он не является частным
	 * - Тэг атрибута из справочника (с сохранением VR из \a tag) в случае, если он там найден
	 * - `DCM_UndefinedTagKey`, если тэг не найден в справочнике.
	 */
	DcmTag findRealPrivateTag(const DcmTag& tag) const noexcept;

public: // Синтаксис передачи

	/** Проверяет возможность конвертации датасета в указанный синтаксис передачи \a xfer.
	 * \param xfer Проверяемый синтаксис передачи
	 * \param xferOrig Оригинальный синтаксис. Если не указан, то используется текущий #getCurrentXfer.
	 * \return true если перекодирование возможно, false в противном случае
	 */
	bool canWriteXfer(const E_TransferSyntax xfer, const E_TransferSyntax xferOrig = EXS_Unknown) const;

	/** Возвращает синтаксис передачи, в котором датасет был прочитан из файла или из памяти.
	 *
	 * Если датасет не инициализирован или не является корневым, то возвращается EXS_LittleEndianExplicit
	 *
	 * \return Синтаксис передачи
	 */
	E_TransferSyntax getOriginalXfer() const;

	/** Возвращает синтаксис передачи, в котором датасет находится в текущий момент
	 *
	 * Если датасет не инициализирован или не является корневым, то возвращается EXS_LittleEndianExplicit
	 * \return Синтаксис передачи
	 */
	E_TransferSyntax getCurrentXfer() const;

	/** Проверяет факт того, что операция изменения синтаксиса передачи может занять продолжительное время.
	 *
	 * С помощью этой функции приложение может понять, что вызов #changeTransferSyntax следует вынести в отдельный
	 * поток.
	 *
	 * \param targetXferUid Целевой синтаксис передачи
	 * \return
	 * - true, если текущий синтаксис сжатый (кроме Deflate) и/или требуется преобразовать в сжатый (кроме Deflate).
	 * - false, если текущий не сжатый или Deflate И целевой не сжатый или Deflate. Также, если невозможно сжать или
	 *   разжать
	 */
	bool isLongRunnningTransferSyntaxChange(const DicomUid& targetXferUid) const;

	/** Производит изменение синтаксиса передачи датасета включая сжатие и/или распаковку доступными кодеками.
	 *
	 * Все операции, кроме Deflate сжатия происходят внутри этой функции, блокируя поток. Рекомендуется ее вызывать из
	 * отдельного потока, т.к. распаковка и упаковка может занять продолжительное время (см.
	 * #isLongRunnningTransferSyntaxChange)
	 *
	 * \param targetXferUid Целевой синтаксис передачи
	 * \param compressOptions Опции сжатия датасета:
	 * - Для всех JPEG lossy:
	 *  - 0: качество = 30-100 (умолчание: 90)
	 * - Для всех JPEG lossless:
	 *  - 0: prediction (умолчание: 1)
	 *  - 1: pointTransform (умолчание: 0)
	 * - Для Deflate:
	 *  - 0: качество сжатия = 0-9 (умолчание 6). Внимание!!! Эта опция
	 *       выставляет глобальную переменную DCMTK! Само сжатие произойдет в
	 *       момент потоковой передачи или сохранения датасета.
	 * - Для JPEG-LS Near Lossless:
	 *  - 0: отклонение = 1-65535 (умолчание: 2)
	 *
	 * Функция оставляет пиксельные данные только для указанного синтаксиса передачи и удаляет (если они есть) данные
	 * оригинального и промежуточного(если требовалась распаковка с последующей упаковкой) синтаксисов.
	 *
	 * \return
	 * - Код ошибки, если произошли ошибки
	 * - capTrue, если синтаксис передачи изменен.
	 * - capFalse, если датасет уже находится в запрашиваемом синтаксисе передачи.
	 */
	CAPRESULT changeTransferSyntax(const DicomUid& targetXferUid,
								   const QVector<int>& compressionOptions = QVector<int>()) const;

	/** Производит изменение синтаксиса передачи датасета включая сжатие и/или распаковку доступными кодеками.
	 *
	 * Описание см. в #changeTransferSyntax(const DicomUid&, const QVector<int>&)
	 */
	CAPRESULT changeTransferSyntax(E_TransferSyntax targetXfer,
								   const QVector<int>& compressionOptions = QVector<int>()) const;


public: // Информация об атрибутах

	/** Проверяет наличие атрибута \a tag в датасете
	 *
	 * \param tag Тэг искомого атрибута
	 * \param searchInSequences Производить ли поиск во вложенных датасетах (сиквенсах)
	 * \return true, если атрибут найден
	 */
	bool contains(const DcmTagKey& tag, bool searchInSequences = false) const noexcept;

	/** Проверяет наличие атрибута с тэгом \a tag в датасете и наличие у него не пустого значения.
	 *
	 * \param tag Тэг искомого атрибута
	 * \param searchInSequences Производить ли поиск во вложенных датасетах (сиквенсах)
	 * \return true, если атрибут найден и содержит хотя бы одно не пустое значение.
	 */
	bool containsNonEmpty(const DcmTagKey& tag, bool searchInSequences = false) const noexcept;

	/** Возвращает VR (Value Representation) атрибута
	 * \return VR атрибута или `DcmEVR::EVR_invalid`, если не найден.
	 */
	DcmEVR getAttributeVR(const DcmTagKey& tag) const noexcept;

	/** Возвращает VM (Value Multiplicity) атрибута
	 * \return VR атрибута или 0, если не найден.
	 */
	int getAttributeVM(const DcmTagKey& tag) const noexcept;

	/** Возвращает количество значений атрибута.
	 *
	 * В отличии от VR:
	 * - Если значение атрибута пустое, то возвращается 0.
	 * - Для VR = SQ возвращается количество элементов.
	 *
	 * \return Количество значений атрибута или 0, если не найден.
	 */
	int getAttributeNumberOfValues(const DcmTagKey& tag) const noexcept;

	/** Возвращает тэг атрибута по индексу \a index
	 * \return Тэг атрибута или `DCM_UndefinedTagKey`, если индекс вне диапазона 0 .. (#count() - 1).
	 */
	DcmTag getAttributeTagAt(int index) const noexcept;

	/** Возвращает количество атрибутов в датасете */
	int count() const noexcept;

	/** Удаляет аттрибут \a tag из датасета
	 * \return true - атрибут удален. false - не найден
	 */
	bool remove(const DcmTagKey& tag) noexcept;

public: // Обработка множества атрибутов

	/** Отладочная функция для вывода содержимого датасета в лог */
	void print(std::ostream& ostrm, int indentLevel = 0) const;

	/** Устанавливает атрибуты датасета как метаинформацию о другом датасете.
	 *
	 * Ожидается, что датасет должен быть типа #Type::MetaInfo.
	 *
	 * Метаинформация используется при записи датасета в файл в качестве заголовка, в котором указывается синтаксис
	 * передачи, тип объекта и другие атрибуты.
	 *
	 * Эта функция заполняет:
	 * - File Meta Information Version (0002,0001)
	 * - Media Storage SOP Class UID (0002,0002) Если в \a dataset указан SOP Class UID (0008,0016).
	 * - Media Storage SOP Instance UID (0002,0003). Если в \a dataset есть SOP Instance UID (0008,0018).
	 * - Transfer Syntax UID (0002,0010)
	 * - Implementation Class UID (0002,0012)
	 * - Implementation Version UID (0002,0013)
	 * - Source Application Entity Title (0002,0016). Если в \a dataset есть PR_TAG_INSTANCE_ORIGINATOR_AE_TITLE
	 * (0045,1005).
	 * - Sending Application Entity Title (0002,0017). Значение #DicomConnectionBase::peerIpAddress, если указан \a con.
	 * - Receiving Application Entity Title (0002,0018). Значение #DicomConnectionBase::localIpAddress, если указан \a
	 * con.
	 *
	 * \param dataset Датасет для которого этот датасет должен хранить метаинформацию.
	 * \param con (опционально) Параметры подключения по которым получен датасет \a dataset.
	 *
	 * \return capOk в случае успеха, другой код в случае ошибки.
	 */
	CAPRESULT fillMetaInfo(const CDataset& dataset, const DicomConnectionBase* con = nullptr);

	/** Удаляет из датасета тэги с номером группы < 0x0008.
	 *
	 * Функция предназначена для обычных датасетов и не должна применяться для датасетов команд DIMSE и для
	 * датасетов с метаинформацией о файле.
	 *
	 * Замечание:
	 * - группы 0, 2, 4, 6 - служебные, используемые в заголовках файлов и DIMSE командах и не должны появляться в
	 * датасетах!
	 * - группы 1, 3, 5, 7 - запрещены к использованию (DICOM PS3.5 7.1 Data Elements)
	 */
	bool stripSpecialTags();

	/** Возвращает список тэгов атрибутов датасета
	 *
	 * \note
	 * При вызове этого метода флаг #Flags::SkipServiceTags не учитывается!
	 *
	 * \param skipServiceTags Исключать служебные атрибуты из списка (см. флаг #Flags::SkipServiceTags)
	 */
	QVector<DcmTag> getTags(bool skipServiceTags = false) const;

	/** Перемещает все атрибуты в другой датасет.
	 *
	 * Атрибуты перемещаются без учета кодировки, временной зоны и синтаксисов передачи.
	 *
	 * Метод перемещает все атрибуты включая служебные (см. #getTags), что может вызвать неправильную обработку
	 * ранее существовавших в датасете назначения атрибутов.
	 *
	 * \param target Датасет назначения.
	 * \param bReplace Заменять ли в датасете назначения атрибуты на новые. Атрибуты из текущего датасета будут
	 * удалены в любом случае.
	 */
	CAPRESULT moveAllAttributesTo(CDataset& target, bool bReplace = true);

	/** Копирует все атрибуты в другой датасет.
	 *
	 * Атрибуты копируются без учета кодировки, временной зоны и синтаксисов передачи.
	 *
	 * Метод копирует все атрибуты включая служебные (см. #getTags), что может вызвать неправильную обработку
	 * ранее существовавших в датасете назначения атрибутов.
	 *
	 * \param target Датасет назначения.
	 * \param bReplace Заменять ли в датасете назначения атрибуты на новые.
	 */
	CAPRESULT copyAllAttributesTo(CDataset& target, bool bReplace = true) const;

	/** Перемещает атрибут в другой датасет с заменой.
	 *
	 * Атрибут перемещаются без учета кодировки, временной зоны и синтаксисов передачи.
	 *
	 * \param tag Тэг перемещаемого атрибута.
	 * \param target Целевой датасет.
	 * \return
	 * - #capOk в случае успеха
	 * - #capElementNotFound, если искомый атрибут \a tag не найден
	 * - Другой код, возвращенный из DCMTK
	 */
	CAPRESULT moveAttributeTo(const DcmTagKey& tag, CDataset& target);

	/** Копирует атрибут в другой датасет с заменой.
	 *
	 * Атрибут копируются без учета кодировки, временной зоны и синтаксисов передачи.
	 *
	 * \param tag Тэг перемещаемого атрибута.
	 * \param target Целевой датасет
	 * \return
	 * - #capOk в случае успеха
	 * - #capElementNotFound, если искомый атрибут \a tag не найден
	 * - Другой код, возвращенный из DCMTK
	 */
	CAPRESULT copyAttributeTo(const DcmTagKey& tag, CDataset& target) const;

	/** Записывает атрибуты из пар ключ-значение
	 *
	 * Поддерживаемые типы значений см. #put(const DcmTag&, const QVariant&, bool).
	 *
	 * Если произошла ошибка, то в датасете останется "частично" записанный набор атрибутов.
	 *
	 * Следующие атрибуты записываются в первую очередь, т.к. могут повлиять на разбор остальных:
	 * - Timezone Offset from UTC (0008,0201)
	 * - Specific Character Set (0008,0005)
	 *
	 * Следующие атрибуты игнорируются:
	 * - Длины групп (gggg,0000)
	 * - Резервации частных атрибутов (gggg,00ee), если установлен флаг #Flags::ResolvePrivateTags
	 *
	 * \return
	 * - #capOk В случае успеха
	 * - #capDicomDimseTypeIncompatibleWithVr - Невозможно преобразовать значение атрибута (несовместимый тип)
	 * - #capFormatError - Невозможно разобрать значение атрибута в соответствии с форматом (обычно для DA, TM, DT или
	 *   числовых типов).
	 * - Другой код в случае ошибок обработки Private Creator или ошибок в DCMTK
	 */
	CAPRESULT putAttributes(const std::initializer_list<std::pair<DcmTag, QVariant>>& values);

	/** Записывает атрибуты из карты QVariant
	 *
	 * Поддерживаемые типы значений см. #put(const DcmTag&, const QVariant&, bool).
	 *
	 * Если произошла ошибка, то в датасете останется "частично" записанный набор атрибутов.
	 *
	 * Следующие атрибуты записываются в первую очередь, т.к. могут повлиять на разбор остальных:
	 * - Timezone Offset from UTC (0008,0201)
	 * - Specific Character Set (0008,0005)
	 *
	 * Следующие атрибуты игнорируются:
	 * - Длины групп (gggg,0000)
	 * - Резервации частных атрибутов (gggg,00ee), если установлен флаг #Flags::ResolvePrivateTags
	 *
	 * \return
	 * - #capOk В случае успеха
	 * - #capDicomDimseTypeIncompatibleWithVr - Невозможно преобразовать значение атрибута (несовместимый тип)
	 * - #capFormatError - Невозможно разобрать значение атрибута в соответствии с форматом (обычно для DA, TM, DT или
	 *   числовых типов).
	 * - Другой код в случае ошибок обработки Private Creator или ошибок в DCMTK
	 */
	CAPRESULT putAttributes(const DatasetVariantMap& values);

	/** Записывает атрибуты из QJsonObject
	 *
	 * Ключи ожидаются в одном из форматов, поддерживаемых в в #CDicomTagInfo::parseUserInput.
	 *
	 * Поддерживаемые типы значений см. в #put(const DcmTag&, const QJsonValue&, bool).
	 *
	 * Если произошла ошибка, то в датасете останется "частично" записанный набор атрибутов.
	 *
	 * Следующие атрибуты записываются в первую очередь, т.к. могут повлиять на разбор остальных:
	 * - Timezone Offset from UTC (0008,0201)
	 * - Specific Character Set (0008,0005)
	 *
	 * Следующие атрибуты игнорируются:
	 * - Длины групп (gggg,0000)
	 * - Резервации частных атрибутов (gggg,00ee), если установлен флаг #Flags::ResolvePrivateTags
	 *
	 * \return
	 * - #capOk В случае успеха
	 * - #capInvalidArg Если невозможно разобрать ключ атрибута
	 * - #capDicomDimseTypeIncompatibleWithVr - Невозможно преобразовать значение атрибута (несовместимый тип)
	 * - #capFormatError - Невозможно разобрать значение атрибута в соответствии с форматом (обычно для DA, TM, DT или
	 *   числовых типов).
	 * - Другой код в случае ошибок обработки Private Creator или ошибок в DCMTK
	 */
	CAPRESULT putAttributes(const QJsonObject& object);

	/** Возвращает список атрибутов в виде карты QVariant
	 *
	 * Формат значений см. в #get(const DcmTagKey&, QVariant&, bool)
	 *
	 * Атрибут датасета в возвращаемых данных может отсутствовать если:
	 * - Возникла ошибка его извлечения
	 * - Это пиксельные данные
	 * - Атрибут является длинной группы
	 * - (в случае \a skipServiceTags или #Flags::SkipServiceTags) Атрибут является служебным
	 * - (в случае #Flags::ResolvePrivateTags) Тэг частного атрибута не найден в справочнике
	 * - (в случае #Flags::ResolvePrivateTags) Атрибут является резервацией для частных атрибутов
	 */
	DatasetVariantMap getAttributesAsMap(bool skipServiceTags = false) const;

	/** Возвращает список атрибутов в виде QJsonObject
	 *
	 * Ключи объекта кодируются в виде текста в формате `(gggg,eeee)` (в верхнем регистре)
	 *
	 * Формат значений см. в #get(const DcmTagKey&, QJsonValue&)
	 *
	 * Атрибут датасета в возвращаемых данных может отсутствовать если:
	 * - Возникла ошибка его извлечения
	 * - Это пиксельные данные
	 * - Атрибут является длинной группы
	 * - (в случае \a skipServiceTags или #Flags::SkipServiceTags) Атрибут является служебным
	 * - (в случае #Flags::ResolvePrivateTags) Тэг частного атрибута не найден в справочнике
	 * - (в случае #Flags::ResolvePrivateTags) Атрибут является резервацией для частных атрибутов
	 */
	QJsonObject getAttributesAsJson(bool skipServiceTags = false) const;

public: // Чтение байтового представление атрибута

	/** Возвращает сырое значение атрибута в виде текста QByteArray
	 *
	 * Поддерживаемые VR:
	 * - Все текстовые без преобразования кодировки:
	 *   AE, AS, CS, DA, DS, DT, IS, LO, LT, PN, SH, ST, TM, UC, UI, UT, UR
	 * - Двоичные в порядке следования байт этой машины:
	 *   FL, FD, OB, OD, OF, OL, OV, OW, SL, SS, SV, UL, UN, US, UV
	 *
	 * \return
	 * - #capElementNotFound - Атрибут не найден в датасете
	 * - #capDicomDimseTypeIncompatibleWithVr - VR несовместим с типом данных.
	 * - Другой код в случае ошибок обработки Private Creator или ошибок в DCMTK
	 */
	CAPRESULT getBytes(const DcmTagKey& tag, QByteArray& rv, bool bRemove = false) const;

	/** Возвращает сырое значение атрибута в виде текста std::string_view
	 *
	 * Поддерживаемые VR:
	 * - Все текстовые без преобразования кодировки:
	 *   AE, AS, CS, DA, DS, DT, IS, LO, LT, PN, SH, ST, TM, UC, UI, UT, UR
	 * - Двоичные в порядке следования байт этой машины:
	 *   FL, FD, OB, OD, OF, OL, OV, OW, SL, SS, SV, UL, UN, US, UV
	 *
	 * Возвращенным значением из этой функции можно пользоваться только до момента изменения или уничтожения датасета,
	 *  т.к. оно ссылается на память в датасет DCMTK.
	 *
	 * \return
	 * - #capElementNotFound - Атрибут не найден в датасете
	 * - #capDicomDimseTypeIncompatibleWithVr - VR несовместим с типом данных.
	 * - Другой код в случае ошибок обработки Private Creator или ошибок в DCMTK
	 */
	CAPRESULT getBytes(const DcmTagKey& tag, std::string_view& rv) const;

	/** Возвращает сырое значение атрибута в виде текста const char*
	 *
	 * Поддерживаемые VR: Все текстовые без преобразования кодировки:
	 *   AE, AS, CS, DA, DS, DT, IS, LO, LT, PN, SH, ST, TM, UC, UI, UT, UR
	 *
	 * Возвращенным значением из этой функции можно пользоваться только до момента изменения или уничтожения датасета,
	 * т.к. оно ссылается на память в датасет DCMTK.
	 *
	 * \return
	 * - #capElementNotFound - Атрибут не найден в датасете
	 * - #capFormatError - Внутреннее представление значения атрибута не завершается нулевым байтом или нулевой байт
	 *   содержится внутри текста.
	 * - #capDicomDimseTypeIncompatibleWithVr - VR несовместим с типом данных.
	 * - Другой код в случае ошибок обработки Private Creator или ошибок в DCMTK
	 */
	CAPRESULT getBytes(const DcmTagKey& tag, const char*& rv) const;

	/** Возвращает все значения атрибута \a tag в виде текста
	 * \tparam T Тип возвращаемого значения (QByteArray, std::string_view или const char*)
	 * \param tag Тэг значения
	 * \return Запрошенное значение
	 */
	template<class T = std::string_view> inline T getBytes(const DcmTag& tag) const;

	/** Возвращает все значения атрибута \a tag в виде текста
	 * \tparam T Тип возвращаемого значения (QByteArray, std::string_view или const char*)
	 * \param tag Тэг значения
	 * \param def Значение по-умолчанию, возвращаемое если по каким либо причинам вызов #getBytes завершился ошибкой.
	 * \return Запрошенное значение
	 */
	template<class T = std::string_view> inline T getBytesDefaulted(const DcmTag& tag, T&& defaultValue) const;

public: // Чтение текстового представления атрибута

	/** Возвращает все значения атрибута \a tag в виде QString
	 *
	 * Поддерживаемые VR:
	 * - Текстовые с кодировкой #encoding: LO, LT, PN, SH, ST, UC, UT
	 * - Текстовые с кодировкой "latin1": AE, AS, CS, DA, DS, DT, IS, TM, UI, UR
	 * - Бинарные c преобразованием число => текст: FL, FD, OB, OD, OF, OL, OV, OW, SL, SS, SV, UL, UN, US, UV
	 *
	 * Если атрибут содержит несколько значений, то они будут разделены стандартным разделителем "\\"
	 * (или #configPnDelimiter в случае, если VR == PN)
	 *
	 * \return
	 * - #capElementNotFound - Атрибут не найден.
	 * - #capDicomDimseTypeIncompatibleWithVr - Невозможно преобразовать значение атрибута в текст. Например, VR = SQ.
	 * - Другой код в случае ошибок обработки Private Creator или ошибок в DCMTK
	 */
	CAPRESULT getText(const DcmTagKey& tag, QString& rv, bool bRemove = false) const;

	/** Возвращает все значения атрибута \a tag в виде QByteArray
	 *
	 * Поддерживаемые VR:
	 * - Текстовые с перекодировкой из #encoding в UTF-8: LO, LT, PN, SH, ST, UC, UT
	 * - Текстовые в исходном виде "latin1": AE, AS, CS, DA, DS, DT, IS, TM, UI, UR
	 * - Бинарные c преобразованием число => текст: FL, FD, OB, OD, OF, OL, OV, OW, SL, SS, SV, UL, UN, US, UV
	 *
	 * Если атрибут содержит несколько значений, то они будут разделены стандартным разделителем "\\"
	 * (или pnDelimiter в случае, если VR == PN)
	 *
	 * \return
	 * - #capElementNotFound - Атрибут не найден.
	 * - #capDicomDimseTypeIncompatibleWithVr - Невозможно преобразовать значение атрибута в текст. Например, VR = SQ.
	 * - Другой код в случае ошибок обработки Private Creator или ошибок в DCMTK
	 */
	CAPRESULT getText(const DcmTagKey& tag, QByteArray& rv, bool bRemove = false) const;

	/** Возвращает все значения атрибута \a tag в виде текста
	 * \tparam T Тип возвращаемого значения (QByteArray, std::string_view или const char*)
	 * \param tag Тэг значения
	 * \return Запрошенное значение
	 */
	template<class T = QString> inline T getText(const DcmTag& tag) const;

	/** Возвращает все значения атрибута \a tag в виде текста
	 * \tparam T Тип возвращаемого значения (QByteArray, std::string_view или const char*)
	 * \param tag Тэг значения
	 * \param def Значение по-умолчанию, возвращаемое если по каким либо причинам вызов #getBytes завершился ошибкой.
	 * \return Запрошенное значение
	 */
	template<class T = QString> inline T getTextDefaulted(const DcmTag& tag, T&& defaultValue) const;

public: // Особые методы работы с сиквенсами
	/** Возвращает существующий сиквенс или ново созданный, при его отсутствии в датасете
	 *
	 * В случае ошибок работы с DCMTK, возвращается нулевой сиквенс.
	 */
	CDatasetSequence getOrCreateSequence(const DcmTagKey& tag);

	/** Возвращает существующий элемент сиквенса или ново созданный, при его отсутствии в датасете.
	 *
	 * Метод может создать несколько "пустых" элементов перед указанным индексом.
	 *
	 * В случае ошибок работы с DCMTK, возвращается нулевой датасет.
	 */
	CDataset getOrCreateSequenceItem(const DcmTagKey& tag, int index = 0);

	/** Возвращает существующий элемент сиквенса.
	 *
	 * Если элемент не найден, то возвращается нулевой датасет.
	 */
	CDataset getSequenceItem(const DcmTagKey& tag, int index = 0) const;

	/** Возвращает количество элементов в сиквенса или 0, если сиквенс не найден */
	int getSequenceSize(const DcmTagKey& tag) const;

	/** Возвращает QDateTime собранный из отдельных значений DA и TM.
	 *
	 * Метод устанавливает офсет времени для \a rvDateTime, если он задан в #timezoneOffsetFromUtc\
	 * \return
	 * - #capElementNotFound - Один из атрибутов не найден
	 * - #capDicomDimseTypeIncompatibleWithVr - Один из атрибутов имеет неожиданный VR
	 * - #capFormatError - Невозможно разобрать дату или время из значений атрибутов.
	 * - Другой код в случае ошибок обработки Private Creator или ошибок в DCMTK
	 */
	CAPRESULT getSeparatedDateAndTime(const DcmTagKey& dateTag, const DcmTagKey& timeTag, QDateTime& rvDateTime) const;

public: // Чтение всех значений атрибута целиком

	/** Возвращает существующий в датасете сиквенс.
	 *
	 * Поддерживаемые VR:
	 * - SQ
	 *
	 * \return
	 * - #capElementNotFound - Атрибут не найден.
	 * - #capDicomDimseTypeIncompatibleWithVr - VR атрибута в датасете не равен `SQ`.
	 * - Другой код в случае ошибок обработки Private Creator или ошибок в DCMTK
	 */
	CAPRESULT get(const DcmTagKey& tag, CDatasetSequence& rv, bool bRemove = false) const;

	/** Возвращает значение атрибута в виде QVariant
	 *
	 * Это значение отражает реальное значение поля датасета без дополнительных конвертаций.
	 *
	 * Если предполагается, что данные датасета должны быть переданы в БД или получены из нее, то следует использовать
	 * класс #CDicomAttributesMapper.
	 *
	 * Общие правила, если в конкретном правиле не сказано иное:
	 * - "пустые" значения возвращаются как невалидный QVariant (.isNull() = true, .isValid() = false).
	 * - #Flags::StrictMode сбрасывается на время чтения атрибута, т.к. некоторые преобразования значений с "потерями".
	 *
	 * Результирующие выходные типы для VR:
	 * - `AE` - #DicomAe (всегда в Latin1) или `QVector<DicomAe>`.
	 * - `AS` - `QByteArray` или `QVector<QByteArray>`
	 * - `AT` - `DcmTagKey` или `QVector<DcmTagKey>`
	 * - `CS` - `QByteArray` или `QVector<QByteArray>`
	 * - `DA` - `QByteArray`, `QDate`, `QPair<QDate, QDate>`, `DicomDate`, `DicomDateRange`, `QVector` от них -
	 *          см. детали в #CDicom::ParseQtDates и #CDicom::ParseDicomDates.
	 * - `DS` - `double` или `QVector<double>`.
	 * - `DT` - `QByteArray`, `QDateTime`, `QPair<QDateTime, QDateTime>`, `DicomDateTime`, `DicomDateTimeRange`,
	 *          `QVector` от них - см. детали в #CDicom::ParseQtDates и #CDicom::ParseDicomDates.
	 * - `FL` - `float` или `QVector<float>`
	 * - `FD` - `double` или `QVector<double>`
	 * - `IS` - `qint32` или `QVector<qint32>`
	 * - `LO` - `QString` или `QVector<QString>`
	 * - `LT` - `QString`
	 * - `OB` - `QByteArray` с двоичным содержимым
	 * - `OD` - `QByteArray` с двоичным содержимым. Длинна кратна 8. Порядок байт - этот хост.
	 * - `OF` - `QByteArray` с двоичным содержимым. Длинна кратна 4. Порядок байт - этот хост.
	 * - `OL` - `QByteArray` с двоичным содержимым. Длинна кратна 4. Порядок байт - этот хост.
	 * - `OV` - `QByteArray` с двоичным содержимым. Длинна кратна 8. Порядок байт - этот хост.
	 * - `OW` - `QByteArray` с двоичным содержимым. Длинна кратна 2. Порядок байт - этот хост.
	 * - `PN` - `QString` или `QVector<QString>`
	 * - `SH` - `QString` или `QVector<QString>`
	 * - `SL` - `qint32` или `QVector<qint32>`
	 * - `SQ` - `QVector<DatasetVariantMap>`. Формат см. в #getAttributesAsMap(bool)
	 * - `SS` - `qint16` или `QVector<qint16>`
	 * - `ST` - `QString`
	 * - `SV` - `qint64` или `QVector<qint64>`
	 * - `TM` - `QByteArray`, `QTime`, `QPair<QTime, QTime>`, `DicomTime`, `DicomTimeRange`, `QVector` от них -
	 *          см. детали в #CDicom::ParseQtDates и #CDicom::ParseDicomDates.
	 * - `UC` - `QString` или `QVector<QString>`
	 * - `UI` - #DicomUid (`QByteArray`) или `QVector<DicomUid>`
	 * - `UL` - `quint32` или `QVector<quint32>`
	 * - `UN` - `QByteArray` с двоичным содержимым.
	 * - `UR` - `QByteArray`
	 * - `US` - `quint16` или `QVector<quint16>`
	 * - `UT` - `QString`
	 * - `UV` - `quint64` или `QVector<quint64>`
	 */
	CAPRESULT get(const DcmTagKey& tag, QVariant& rvValue, bool bRemove = false) const;

	/** Возвращает значение атрибута в виде QJsonValue
	 *
	 * Внимание! Этот формат записи JSON не совместим с DICOM Web! Он используется только для внутренних сервисов
	 * и скриптов.
	 *
	 * Общие правила, если в конкретном правиле не сказано иное:
	 * - "пустые" значения возвращаются как `null`.
	 * - Для `string` с VR для которого не применим #encoding, текст обрабатывается в кодировке `Latin1`.
	 * - #Flags::StrictMode сбрасывается на время чтения атрибута, т.к. некоторые преобразования значений с "потерями".
	 *
	 * Результирующие выходные типы для VR:
	 * - `AE` - `array` или `string`.
	 * - `AS` - `array` или `string`.
	 * - `AT` - `array` или `string` в формате `(gggg,eeee)`
	 * - `CS` - `array` или `string`
	 * - `DA` - `array`, `string` или `object`. См. детали в #Flags::ParseQtDates
	 * - `DS` - `array` или `double`.
	 * - `DT` - `array`, `string` или `object`. См. детали в #Flags::ParseQtDates
	 * - `FL` - `array` или `double`
	 * - `FD` - `array` или `double`
	 * - `IS` - `array` или `double`
	 * - `LO` - `array` или `string`
	 * - `LT` - `string`.
	 * - `OB` - `string` с base64 двоичного содержимого.
	 * - `OD` - `string` с base64 двоичного содержимого. Длинна кратна 8. Порядок байт - этот хост.
	 * - `OF` - `string` с base64 двоичного содержимого. Длинна кратна 4. Порядок байт - этот хост.
	 * - `OL` - `string` с base64 двоичного содержимого. Длинна кратна 4. Порядок байт - этот хост.
	 * - `OV` - `string` с base64 двоичного содержимого. Длинна кратна 8. Порядок байт - этот хост.
	 * - `OW` - `string` с base64 двоичного содержимого. Длинна кратна 2. Порядок байт - этот хост.
	 * - `PN` - `array` или `string`.
	 * - `SH` - `array` или `string`.
	 * - `SL` - `array` или `double`.
	 * - `SQ` - `array`. Формат объектов внутри `array`: см. в #getAttributesAsJson(bool)
	 * - `SS` - `array` или `double`
	 * - `ST` - `string`
	 * - `SV` - `array`, `string` с числом.
	 * - `TM` - `array`, `string` или `object`. См. детали в #Flags::ParseQtDates
	 * - `UC` - `array` или `string`.
	 * - `UI` - `array` или `string`.
	 * - `UL` - `array` или `string` с числом.
	 * - `UN` - `string` с base64 двоичного содержимого.
	 * - `UR` - `string`
	 * - `US` - `array` или `double`.
	 * - `UT` - `string`.
	 * - `UV` - `array` или `string` с числом.
	 */
	CAPRESULT get(const DcmTagKey& tag, QJsonValue& rvValue, bool bRemove = false) const;

	/** Возвращает все значения атрибута \a tag в виде списка float (DS, FL, OF)
	 *
	 * Поддерживаемые VR:
	 * - Полноценно: DS, FL, OF
	 * - Конвертация при отсутствии #Flags::StrictMode: FD, IS, OD, SL, SS, SV, UL, US, UV
	 *
	 * \param tag Идентификатор атрибута. Если указан частный атрибут и #isResolvingPrivateTags == \c true,
	 * то идентификатор проходит через #resolvePrivateTag.
	 * \param rv Возвращаемое значение.
	 * \param bRemove Удалять ли атрибут из датасета при успешном возвращении значения.
	 * \return
	 * - #capElementNotFound - Атрибут не найден в датасете
	 * - #capFormatError - Невозможно преобразовать число без потерь или исходный текст в тип возвращаемого результата.
	 * - #capDicomDimseTypeIncompatibleWithVr - VR несовместим с типом данных (также, если установлен #Flag::StrictMode
	 * и VR не из списка поддерживаемых полноценно).
	 * - Другой код в случае ошибок обработки Private Creator или ошибок в DCMTK
	 */
	CAPRESULT get(const DcmTagKey& tag, QVector<float>& rv, bool bRemove = false) const;

	/** Возвращает все значения атрибута \a tag в виде списка double (DS, FD, OD)
	 *
	 * Поддерживаемые VR:
	 * - Полноценно: DS, FD, OD
	 * - Конвертация при отсутствии #Flags::StrictMode: FL, IS, OF, SL, SS, SV, UL, US, UV
	 *
	 * \sa get(const DcmTagKey&, QVector<float>&, bool)
	 */
	CAPRESULT get(const DcmTagKey& tag, QVector<double>& rv, bool bRemove = false) const;

	/** Возвращает все значения атрибута \a tag в виде списка quint8 (OB, UN)
	 *
	 * Поддерживаемые VR:
	 * - Полноценно: OB, UN
	 * - Конвертация при отсутствии #Flags::StrictMode: DS, FD, FL, IS, SL, SS, SV, UL, US, UV
	 *
	 * \sa get(const DcmTagKey&, QVector<float>&, bool)
	 */
	CAPRESULT get(const DcmTagKey& tag, QVector<quint8>& rv, bool bRemove = false) const;

	/** Возвращает все значения атрибута \a tag в виде списка qint16 (SS)
	 *
	 * Поддерживаемые VR:
	 * - Полноценно: SS
	 * - Конвертация при отсутствии #Flags::StrictMode: DS, FD, FL, IS, SL, SV, UL, US, UV
	 *
	 * \sa get(const DcmTagKey&, QVector<float>&, bool)
	 */
	CAPRESULT get(const DcmTagKey& tag, QVector<qint16>& rv, bool bRemove = false) const;

	/** Возвращает все значения атрибута \a tag в виде списка quint16 (AT, OW, US)
	 *
	 * Поддерживаемые VR:
	 * - Полноценно: AT, OW, US
	 * - Конвертация при отсутствии #Flags::StrictMode: DS, FD, FL, IS, SL, SS, SV, UL, UV
	 *
	 * \sa get(const DcmTagKey&, QVector<float>&, bool)
	 */
	CAPRESULT get(const DcmTagKey& tag, QVector<quint16>& rv, bool bRemove = false) const;

	/** Возвращает все значения атрибута \a tag в виде списка qint32 (IS, SL)
	 *
	 * Поддерживаемые VR:
	 * - Полноценно: IS, SL
	 * - Конвертация при отсутствии #Flags::StrictMode: DS, FD, FL, SS, SV, US, UL, UV
	 *
	 * \sa get(const DcmTagKey&, QVector<float>&, bool)
	 */
	CAPRESULT get(const DcmTagKey& tag, QVector<qint32>& rv, bool bRemove = false) const;

	/** Возвращает все значения атрибута \a tag в виде списка quint32 (OL, UL)
	 *
	 * Поддерживаемые VR:
	 * - Полноценно: OL, UL, AT (group в старших 16 битах, element - в младших)
	 * - Конвертация при отсутствии #Flags::StrictMode: DS, FD, FL, IS, SL, SS, SV, US, UV
	 *
	 * \sa get(const DcmTagKey&, QVector<float>&, bool)
	 */
	CAPRESULT get(const DcmTagKey& tag, QVector<quint32>& rv, bool bRemove = false) const;

	/** Возвращает все значения атрибута \a tag в виде списка qint64 (SV)
	 *
	 * Поддерживаемые VR:
	 * - Полноценно: SV
	 * - Конвертация при отсутствии #Flags::StrictMode: DS, FD, FL, IS, SS, SL, US, UL, UV
	 *
	 * \sa get(const DcmTagKey&, QVector<float>&, bool)
	 */
	CAPRESULT get(const DcmTagKey& tag, QVector<qint64>& rv, bool bRemove = false) const;

	/** Возвращает все значения атрибута \a tag в виде списка quint64 (UV, OV)
	 *
	 * Поддерживаемые VR:
	 * - Полноценно: UV, OV
	 * - Конвертация при отсутствии #Flags::StrictMode: DS, FD, FL, IS, SS, SL, SV, US, UL
	 *
	 * \sa get(const DcmTagKey&, QVector<float>&, bool)
	 */
	CAPRESULT get(const DcmTagKey& tag, QVector<quint64>& rv, bool bRemove = false) const;

	/** Возвращает все значения атрибута \a tag в виде списка DcmTagKey (AT)
	 *
	 * Поддерживается только VR = AT.
	 *
	 * \sa get(const DcmTagKey&, QVector<float>&, bool)
	 */
	CAPRESULT get(const DcmTagKey& tag, QVector<DcmTagKey>& rv, bool bRemove = false) const;

	/** Возвращает все значения атрибута \a tag в виде юникод текста
	 *
	 * Поддерживаемые VR:
	 * - Текстовые с кодировкой #encoding: LO, LT, PN, SH, ST, UC, UT
	 * - Текстовые с кодировкой "latin1": AE, AS, CS, DA, DS, DT, IS, TM, UI, UR
	 * - Бинарные c преобразованием число => текст: FL, FD, OB, OD, OF, OL, OV, OW, SL, SS, SV, UL, UN, US, UV
	 *
	 * \sa get(const DcmTagKey&, QVector<float>&, bool)
	 */
	CAPRESULT get(const DcmTagKey& tag, QVector<QString>& rv, bool bRemove = false) const;

	/** Возвращает все значения атрибута \a tag в виде текста
	 *
	 * Поддерживаемые VR:
	 * - Текстовые с кодировкой #encoding (приведенные в UTF-8): LO, LT, PN, SH, ST, UC, UT
	 * - Текстовые без преобразования кодировки: AE, AS, CS, DA, DS, DT, IS, TM, UI, UR
	 * - Бинарные c преобразованием число => текст: FL, FD, OB, OD, OF, OL, OV, OW, SL, SS, SV, UL, UN, US, UV
	 *
	 * \sa get(const DcmTagKey&, QVector<float>&, bool)
	 */
	CAPRESULT get(const DcmTagKey& tag, QVector<QByteArray>& rv, bool bRemove = false) const;

public: // Получение одного значения атрибута по индексу

	/** Возвращает значение сиквенса \a tag по указанному индексу \a index
	 *
	 * Возвращаемый объект ссылается на этот объект. Если предполагается его "сохранение" и использование независимо
	 * от этого объекта, то необходимо вызвать #CDataset::clone у возвращенного значения.
	 *
	 * Поддерживаемые VR:
	 * - SQ
	 *
	 * \param tag Идентификатор атрибута. Если указан частный атрибут и #isResolvingPrivateTags == \c true,
	 * то идентификатор проходит через #resolvePrivateTag.
	 * \param bRemove Удалять ли весь сиквенс из датасета при успешном возвращении значения. Если это значение \c true,
	 * то возвращаемый \a rv будет владеть DCMTK объектом. Если \c false, то будет только ссылаться без владения.
	 */
	CAPRESULT get(const DcmTagKey& tag, CDataset& rv, bool bRemove = false, int index = 0) const;

	/** Возвращает значение \a index атрибута \a tag в виде float (DS, FL, OF)
	 *
	 * Поддерживаемые VR:
	 * - Полноценно: DS, FL, OF
	 * - Конвертация при отсутствии #Flags::StrictMode: FD, IS, OD, SL, SS, SV, UL, US, UV
	 *
	 * \param tag Идентификатор атрибута. Если указан частный атрибут и #isResolvingPrivateTags == \c true,
	 * то идентификатор проходит через #resolvePrivateTag.
	 * \param rv Возвращаемое значение.
	 * \param bRemove Удалять ли атрибут из датасета при успешном возвращении значения.
	 * \param index Индекс возвращаемого значения
	 * \return
	 * - #capElementNotFound - Атрибут не найден в датасете
	 * - #capIndexOutOfRange - Запрошенный индекс выходит за пределы существующих у атрибута
	 * - #capFormatError - Невозможно преобразовать число без потерь или исходный текст в тип возвращаемого результата.
	 * - #capDicomDimseTypeIncompatibleWithVr - VR несовместим с типом данных (также, если есть флаг #Flags::StrictMode
	 *   и VR не из списка поддерживаемых полноценно)
	 * - Другой код в случае ошибок обработки Private Creator или ошибок в DCMTK
	 */
	CAPRESULT get(const DcmTagKey& tag, float& rv, bool bRemove = false, int index = 0) const;

	/** Возвращает значение \a index атрибута \a tag в виде double (DS, FD, OD)
	 *
	 * Поддерживаемые VR:
	 * - Полноценно: DS, FD, OD
	 * - Конвертация при отсутствии #Flags::StrictMode: FL, IS, OF, SL, SS, SV, UL, US, UV
	 *
	 * \sa getValue(const DcmTagKey&, float&, bool, int)
	 */
	CAPRESULT get(const DcmTagKey& tag, double& rv, bool bRemove = false, int index = 0) const;

	/** Возвращает значение \a index атрибута \a tag в виде quint8 (OB, UN)
	 *
	 * Поддерживаемые VR:
	 * - Полноценно: OB, UN
	 * - Конвертация при отсутствии #Flags::StrictMode: DS, FD, FL, IS, SL, SS, SV, UL, US, UV
	 *
	 * \sa getValue(const DcmTagKey&, float&, bool, int)
	 */
	CAPRESULT get(const DcmTagKey& tag, quint8& rv, bool bRemove = false, int index = 0) const;

	/** Возвращает значение \a index атрибута \a tag в виде qint16 (SS)
	 *
	 * Поддерживаемые VR:
	 * - Полноценно: SS
	 * - Конвертация при отсутствии #Flags::StrictMode: DS, FD, FL, IS, SL, SV, UL, US, UV
	 *
	 * \sa getValue(const DcmTagKey&, float&, bool, int)
	 */
	CAPRESULT get(const DcmTagKey& tag, qint16& rv, bool bRemove = false, int index = 0) const;

	/** Возвращает значение \a index атрибута \a tag в виде quint16 (AT, OW, US)
	 *
	 * Поддерживаемые VR:
	 * - Полноценно: AT, OW, US
	 * - Конвертация при отсутствии #Flags::StrictMode: DS, FD, FL, IS, SL, SS, SV, UL, UV
	 *
	 * \sa getValue(const DcmTagKey&, float&, bool, int)
	 */
	CAPRESULT get(const DcmTagKey& tag, quint16& rv, bool bRemove = false, int index = 0) const;

	/** Возвращает значение \a index атрибута \a tag в виде qint32 (IS, SL)
	 *
	 * Поддерживаемые VR:
	 * - Полноценно: IS, SL
	 * - Конвертация при отсутствии #Flags::StrictMode: DS, FD, FL, SS, SV, US, UL, UV
	 *
	 * \sa getValue(const DcmTagKey&, float&, bool, int)
	 */
	CAPRESULT get(const DcmTagKey& tag, qint32& rv, bool bRemove = false, int index = 0) const;

	/** Возвращает значение \a index атрибута \a tag в виде quint32 (OL, UL, AT)
	 *
	 * Поддерживаемые VR:
	 * - Полноценно: OL, UL, AT (group в старших 16 битах, element в младших)
	 * - Конвертация при отсутствии #Flags::StrictMode: DS, FD, FL, IS, SL, SS, SV, US, UV
	 *
	 * \sa getValue(const DcmTagKey&, float&, bool, int)
	 */
	CAPRESULT get(const DcmTagKey& tag, quint32& rv, bool bRemove = false, int index = 0) const;

	/** Возвращает значение \a index атрибута \a tag в виде qint64 (SV)
	 *
	 * Поддерживаемые VR:
	 * - Полноценно: SV
	 * - Конвертация при отсутствии #Flags::StrictMode: DS, FD, FL, IS, SS, SL, US, UL, UV
	 *
	 * \sa getValue(const DcmTagKey&, float&, bool, int)
	 */
	CAPRESULT get(const DcmTagKey& tag, qint64& rv, bool bRemove = false, int index = 0) const;

	/** Возвращает значение \a index атрибута \a tag в виде quint64 (UV, OV)
	 *
	 * Поддерживаемые VR:
	 * - Полноценно: UV, OV
	 * - Конвертация при отсутствии #Flags::StrictMode: DS, FD, FL, IS, SS, SL, SV, US, UL
	 *
	 * \sa getValue(const DcmTagKey&, float&, bool, int)
	 */
	CAPRESULT get(const DcmTagKey& tag, quint64& rv, bool bRemove = false, int index = 0) const;

	/** Возвращает значение \a index атрибута \a tag в виде DcmTagKey (AT)
	 *
	 * Поддерживается только VR = AT.
	 *
	 * \sa getValue(const DcmTagKey&, float&, bool, int)
	 */
	CAPRESULT get(const DcmTagKey& tag, DcmTagKey& rv, bool bRemove = false, int index = 0) const;

	/** Возвращает значение \a index атрибута \a tag в виде QDate (DA)
	 *
	 * Поддерживается только VR = DA.
	 *
	 * \sa getValue(const DcmTagKey&, float&, bool, int)
	 */
	CAPRESULT get(const DcmTagKey& tag, QDate& rv, bool bRemove = false, int index = 0) const;

	/** Возвращает значение \a index атрибута \a tag в виде QTime (TM)
	 *
	 * Поддерживается только VR = TM.
	 *
	 * \sa getValue(const DcmTagKey&, float&, bool, int)
	 */
	CAPRESULT get(const DcmTagKey& tag, QTime& rv, bool bRemove = false, int index = 0) const;

	/** Возвращает значение \a index атрибута \a tag в виде QDateTime (DT)
	 *
	 * Поддерживается только VR = DT.
	 *
	 * \sa getValue(const DcmTagKey&, float&, bool, int)
	 */
	CAPRESULT get(const DcmTagKey& tag, QDateTime& rv, bool bRemove = false, int index = 0) const;

	/** Возвращает значение \a index атрибута \a tag в виде QString
	 *
	 * Поддерживаемые VR:
	 * - Текстовые с кодировкой #encoding: LO, LT, PN, SH, ST, UC, UT
	 * - Текстовые с кодировкой "latin1": AE, AS, CS, DA, DS, DT, IS, TM, UI, UR
	 * - Бинарные c преобразованием число => текст: FL, FD, OB, OD, OF, OL, OV, OW, SL, SS, SV, UL, UN, US, UV
	 *
	 * \sa getValue(const DcmTagKey&, float&, bool, int)
	 */
	CAPRESULT get(const DcmTagKey& tag, QString& rv, bool bRemove = false, int index = 0) const;

	/** Возвращает значение \a index атрибута \a tag в виде QByteArray
	 *
	 * Поддерживаемые VR:
	 * - Текстовые с кодировкой #encoding (приведенные в UTF-8): LO, LT, PN, SH, ST, UC, UT
	 * - Текстовые без преобразования кодировки: AE, AS, CS, DA, DS, DT, IS, TM, UI, UR
	 * - Бинарные c преобразованием число => текст: FL, FD, OB, OD, OF, OL, OV, OW, SL, SS, SV, UL, UN, US, UV
	 *
	 * \sa getValue(const DcmTagKey&, float&, bool, int)
	 */
	CAPRESULT get(const DcmTagKey& tag, QByteArray& rv, bool bRemove = false, int index = 0) const;

	/** Возвращает все значения или первое значение атрибута \a tag (в зависимости от типа T)
	 * \tparam T Тип возвращаемого значения. Должен быть одним из поддерживаемых в методах #get
	 * \param tag Тэг значения
	 * \return Значение атрибута или значение по умолчанию, если атрибут не найден или произошла ошибка извлечения.
	 */
	template<class T> inline T get(const DcmTagKey& tag) const;

	/** Возвращает все значения или первое значение атрибута \a tag (в зависимости от типа T) или значение по умолчанию.
	 * \tparam T Тип возвращаемого значения. Должен быть одним из поддерживаемых в методах #get
	 * \param tag Тэг значения
	 * \param defaultValue Значение по умолчанию, если атрибут не найден или не может быть извлечен.
	 * \return Значение атрибута или значение по умолчанию
	 */
	template<class T> inline T getDefaulted(const DcmTagKey& tag, T&& defaultValue) const;

public: // Запись байтового представление атрибута

	/** Записывает атрибут \a tag с указанным байтовым значением \a val
	 *
	 * Поддерживаемые VR:
	 * - Все текстовые без преобразования кодировки:
	 *   AE, AS, CS, DA, DS, DT, IS, LO, LT, PN, SH, ST, TM, UC, UI, UT, UR
	 * - Двоичные в порядке следования байт этой машины:
	 *   FL, FD, OB, OD, OF, OL, OV, OW, SL, SS, SV, UL, UN, US, UV
	 *
	 * \return
	 * - #capFormatError - Количество байт не кратно длине одного значения двоичного VR.
	 * - #capElementExists - Атрибут уже существует (возможен в случае \a bReplace == \c false)
	 * - #capDicomDimseTypeIncompatibleWithVr - Запись двоичных данных не поддерживается (например, для VR=SQ)
	 * - Другой код в случае ошибок обработки Private Creator или ошибок в DCMTK
	 */
	CAPRESULT putBytes(const DcmTag& tag, const QByteArray& val, bool bReplace = true);

	/** Создает или обновляет атрибут \a tag с указанным байтовым значением \a val
	 * \sa putBytes(const DcmTag&, const QByteArray&, bool);
	 */
	CAPRESULT putBytes(const DcmTag& tag, std::string_view val, bool bReplace = true);

	/** Создает или обновляет атрибут \a tag с указанным байтовым значением \a val
	 *
	 * Поддерживаемые VR: Все текстовые без преобразования кодировки:
	 * AE, AS, CS, DA, DS, DT, IS, LO, LT, PN, SH, ST, TM, UC, UI, UT, UR
	 *
	 * \sa putBytes(const DcmTag&, const QByteArray&, bool);
	 */
	CAPRESULT putBytes(const DcmTag& tag, const char* val, bool bReplace = true);

	/** Создает или обновляет атрибут \a tag с указанным байтовым значением \a val
	 * \sa putBytes(const DcmTag&, const QByteArray&, bool);
	 */
	template <size_t Size>
	inline CAPRESULT putBytes(const DcmTag& tag, const char (&val)[Size], bool bReplace = true);

public: // Запись текстового представление атрибута

	/** Создает или обновляет атрибут \a tag с указанным значением \a val
	 *
	 * Поддерживаемые VR:
	 * - Текстовые с кодировкой #encoding: LO, LT, PN, SH, ST, UC, UT
	 * - Текстовые с кодировкой "latin1": AE, AS, CS, DA, DS, DT, IS, TM, UI, UR
	 * - Бинарные c преобразованием текст => число: FL, FD, OB, OD, OF, OL, OV, OW, SL, SS, SV, UL, UN, US, UV
	 *
	 * \param tag Тэг атрибута. Если указан частный атрибут и #isResolvingPrivateTags == \c true,
	 * то тэг проходит через #reservePrivateTag.
	 * \param val Список значений атрибута, разделенный стандартным разделителем "\\" или #pnDelimiter в
	 * случае, если VR устанавливаемого атрибута = PN.
	 * \param bReplace Заменять ли существующее значение.
	 * \return
	 * - #capElementExists - Атрибут уже существует (если \a bReplace == \c false)
	 * - #capFormatError - Невозможно преобразовать число без потерь или входящий тип данных в VR атрибута.
	 * - #capDicomDimseTypeIncompatibleWithVr - VR несовместим с типом данных (также, если есть флаг #Flags::StrictMode
	 *   и VR не из списка поддерживаемых полноценно).
	 * - Другой код в случае ошибок обработки Private Creator или ошибок в DCMTK
	 */
	CAPRESULT putText(const DcmTag& tag, QStringView val, bool bReplace = true);

	/** Создает или обновляет атрибут \a tag с указанным значением \a val
	 * \sa putText(const DcmTag&, QSrtingView, bool)
	 */
	CAPRESULT putText(const DcmTag& tag, const QString& val, bool bReplace = true);

	/** Создает или обновляет атрибут \a tag с указанным значением \a val
	 *
	 * Поддерживаемые VR:
	 * - Текстовые с перекодировкой UTF-8 => #encoding: LO, LT, PN, SH, ST, UC, UT
	 * - Текстовые без перекодировки: AE, AS, CS, DA, DS, DT, IS, TM, UI, UR
	 * - Бинарные c преобразованием текст => число: FL, FD, OB, OD, OF, OL, OV, OW, SL, SS, SV, UL, UN, US, UV
	 *
	 * \return
	 * - #capElementExists - Атрибут уже существует (если \a bReplace == \c false)
	 * - #capFormatError - Невозможно преобразовать число без потерь или входящий тип данных в VR атрибута.
	 * - #capDicomDimseTypeIncompatibleWithVr - VR несовместим с типом данных (также, если есть флаг #Flags::StrictMode
	 *   и VR не из списка поддерживаемых полноценно).
	 * - Другой код в случае ошибок обработки Private Creator или ошибок в DCMTK
	 */
	CAPRESULT putText(const DcmTag& tag, const QByteArray& val, bool bReplace = true);

	/** Создает или обновляет атрибут \a tag с указанным значением \a val
	 * \sa putText(const DcmTag&, const QByteArray&, bool)
	 */
	CAPRESULT putText(const DcmTag& tag, std::string_view val, bool bReplace = true);

	/** Создает или обновляет атрибут \a tag с указанным значением \a val
	 * \sa putText(const DcmTag&, const QByteArray&, bool)
	 */
	CAPRESULT putText(const DcmTag& tag, const char* val, bool bReplace = true);

	/** Создает или обновляет атрибут \a tag с указанным значением \a val
	 * \sa putText(const DcmTag&, const QByteArray&, bool)
	 */
	template <size_t Size>
	inline CAPRESULT putText(const DcmTag& tag, const char (&val)[Size], bool bReplace = true);

public: // Запись всех значений атрибута целиком

	/** Записывает атрибут с пустым значением
	 *
	 * \param tag Тэг атрибута. Если указан частный атрибут и #isResolvingPrivateTags == \c true,
	 * то тэг проходит через #reservePrivateTag.
	 * \param bReplace Заменять ли существующий атрибут новым.
	 */
	CAPRESULT putEmpty(const DcmTag& tag, bool bReplace = true);

	/** Записывает атрибут с пустым значением
	 *
	 * \param tag Тэг атрибута. Если указан частный атрибут и #isResolvingPrivateTags == \c true,
	 * то тэг проходит через #reservePrivateTag.
	 * \param bReplace Заменять ли существующий атрибут новым.
	 */
	CAPRESULT put(const DcmTag& tag, nullptr_t, bool bReplace = true);

	/** Создает или обновляет атрибут \a tag с указанным значением #CDatasetSequence
	 *
	 * \param tag Тэг атрибута. Если указан частный атрибут и #isResolvingPrivateTags == \c true,
	 * то тэг проходит через #reservePrivateTag.
	 * \param val Устанавливаемый сиквенс. Специфика:
	 * - Если DCMTK объект сиквенса в \a val уже находится в датасете, то вызов игнорируется.
	 * - Если сиквенс не владеет DCMTK объектом, то произойдет глубокая копия всех элементов. В противном случае,
	 *   DCMTK объект сиквенса переносится в текущий датасет и \a val останется пустым.
	 * - У каждого элемента сиквенса происходит преобразование кодировки текста и временной зоны к текущей, если это
	 *   требуется.
	 * - Если тэг сиквенса отличается от \a tag, то происходит создание нового DCMTK сиквенса с переносом к нему
	 *   всех элементов.
	 * \param bReplace Заменять ли существующий сиквенс новым.
	 */
	CAPRESULT put(const DcmTag& tag, CDatasetSequence&& val, bool bReplace = true);

	/** Создает или обновляет атрибут \a tag с указанным значением QJsonValue
	 *
	 * Внимание! Этот формат записи JSON не совместим с DICOM Web! Он используется только для внутренних сервисов
	 * или скриптов.
	 *
	 * Общие правила:
	 * - #Flags::StrictMode сбрасывается на время записи атрибута
	 * - Если объект типа `null`, то атрибут запишется без значений.
	 * - При чтении таймстемпов из JSON без указания временной зоны, принимается текущая не учитывая настройки датасета.
	 *
	 * Поддерживаемые входные типы для VR:
	 * - `AE`, `CS`, `DS`, `IS`, `UR`:
	 *   - `string` записывается в кодировке `latin1`.
	 *   - `double` приводится к тексту (в случае с `IS` - отбрасывается дробная часть).
	 *   - `array`
	 * - `AS`:
	 *   - `string` записывается в кодировке `latin1`.
	 *   - `double` - Преобразуется в текстовый формат "количества лет", если число в пределах 0 .. 999.
	 *   - `array`
	 * - `AT`:
	 *   - `string` разбирается в формате #CDicomTagInfo::fromUserInput
	 *   - `double` приводится к 32-битному бесзнаковому и воспринимается как номер группы и номер элемента.
	 *   - `array`
	 * - `DA`
	 *   - `string` - производится попытка разбора в форматах (в порядке приоритета):
	 *     - YYYY-MM-DD
	 *     - YYYY-MM-DDTHH:mm:ss.SSS±HH:mm с отбрасыванием времени
	 *     - Стандартная запись DA в DICOM
	 *     - Стандартная запись DT в DICOM с отбрасыванием времени
	 *     - Специальная форматы, если #Flags::IsQueryRetrieve установлен:
	 *       - Запись диапазона DA.
	 *       - Запись диапазона DT с отбрасыванием времени.
	 *       - Две двойные кавычки.
	 *   - `double` - Воспринимается как таймстемп в секундах от начала эпохи.
	 *   - `object` с полями "from" и "to" - Разрешен, если #Flags::IsQueryRetrieve установлен. Каждый элемент
	 *     этого объекта обрабатывается по правилам как для всего атрибута DA, но, без учета #Flags::IsQueryRetrieve.
	 *     Два разобранных элемента формируют диапазон значений.
	 *   - `array` - При разборе дочерних элементов массива использование двух двойных кавычек запрещается.
	 * - `DT`
	 *   - `string` - производится попытка разбора в форматах (в порядке приоритета):
	 *     - YYYY-MM-DDTHH:mm:ss.SSS±HH:mm
	 *     - Стандартная запись DT в DICOM
	 *     - Специальная форматы, если #Flags::IsQueryRetrieve установлен:
	 *       - Запись диапазона DT.
	 *       - Две двойные кавычки.
	 *   - `double` - Воспринимается как таймстемп в секундах от начала эпохи.
	 *   - `object` с полями "from" и "to" - Разрешен, если #Flags::IsQueryRetrieve установлен. Каждый элемент
	 *     этого объекта обрабатывается по правилам как для всего атрибута DT, но, без учета #Flags::IsQueryRetrieve.
	 *     Два разобранных элемента формируют диапазон значений.
	 *   - `array` - При разборе дочерних элементов массива использование двух двойных кавычек запрещается.
	 * - `FL`, `FD`, `SL`, `SS`, `SV`, `UL`, `US`, `UV`:
	 *   - `string` разбирается как число с типом VR.
	 *   - `double` приводится к типу VR с проверкой переполнения.
	 *   - `array`
	 * - `LO`, `PN`, `SH`, `UC`:
	 *   - `string` записывается в кодировке #encoding.
	 *   - `double` приводится к тексту.
	 *   - `array`
	 * - `LT`, `ST`, `UT`:
	 *   - `string` записывается в кодировке #encoding.
	 *   - `double` приводится к тексту.
	 * - `OB`, `OD`, `OF`, `OL`, `OV`, `OW`, `UN`:
	 *   - `string` воспринимается как base64 двоичные данные атрибута.
	 * - `SQ`:
	 *   - `object` - Разбирается как один элемент сиквенса
	 *   - `array` - Разбирается как массив сиквенсов
	 * - `TM`:
	 *   - `string` - производится попытка разбора в форматах (в порядке приоритета):
	 *     - HH:mm:ss.SSS
	 *     - YYYY-MM-DDTHH:mm:ss.SSS±HH:mm с отбрасыванием даты
	 *     - Стандартная запись TM в DICOM
	 *     - Стандартная запись DT в DICOM с отбрасыванием даты
	 *     - Специальная форматы, если #Flags::IsQueryRetrieve установлен:
	 *       - Запись диапазона TM.
	 *       - Запись диапазона DT с отбрасыванием даты.
	 *       - Две двойные кавычки.
	 *   - `double` - Воспринимается как таймстемп в секундах от начала эпохи.
	 *   - `object` с полями "from" и "to" - Разрешен, если #Flags::IsQueryRetrieve установлен. Каждый элемент
	 *     этого объекта обрабатывается по правилам как для всего атрибута TM, но, без учета #Flags::IsQueryRetrieve.
	 *     Два разобранных элемента формируют диапазон значений.
	 *   - `array` - При разборе дочерних элементов массива использование двух двойных кавычек запрещается.
	 * - `UI`:
	 *   - `string` - производится попытка разобрать в формате записи QUuid. Если получилось, то QUuid трансформируется
	 *     в OID и записывается. Если не получилось, то текст записывается без изменений в кодировке `latin1`.
	 *   - `array`
	 */
	CAPRESULT put(const DcmTag& tag, const QJsonValue& val, bool bReplace = true);

	/** Создает или обновляет атрибут \a tag с указанным значением QVariant
	 *
	 * Общие правила:
	 * - #Flags::StrictMode сбрасывается на время записи атрибута
	 * - Если .isNull() == true ИЛИ .isValid() == false, то атрибут запишется без значений.
	 * - Если входящий тип nullptr_t, то атрибут запишется без значений.
	 * - Если входящий тип QJSonValue, QJsonObject или QJsonArray, то управление передается в
	 *   #put(const DcmTag&, const QJsonValue&, bool) вне зависимости от VR.
	 * - Если индивидуальный тип данных для VR не подходит, то происходит попытка конвертации QVariant:
	 *   1. Если VR бинарный для пиксельных данных или текстовый без поддержки #encoding, то к QByteArray.
	 *   2. Если VR текстовый с поддержкой #encoding, то к QString.
	 *   3. Если VR бинарный числовой или текстовый числовой, то к типу числа, соответствующему VR.
	 *   4. Если VR допускает QVector, то к QSequentialIterable. Каждый вложенный QVariant обрабатывается по полному
	 *   циклу для VR суммируя все значения в выходном атрибуте. Единственное исключение - запрещаются "вложенные"
	 *   массивы: QVector, QJsonValue с типом `array`, QJsonArray, другой QSequentialIterable.
	 *
	 * Поддерживаемые входные типы для VR:
	 * - `AE`:
	 *   - QByteArray записываются как есть.
	 *   - QString записываются в кодировке `latin1`.
	 *   - quint8, qint16, quint16, qint32, quint32, qint64, quint64, double, float - Записываются в виде текста.
	 *   - QUuid - Записывается в виде текста без фигурных скобок.
	 *   - QVector допускается для всех типов перечисленных выше.
	 * - `AS`:
	 *   - quint8, qint16, quint16, qint32, quint32, qint64, quint64, double, float - Преобразуется в текстовый
	 *     формат "количества лет", если число в пределах 0 .. 999.
	 *   - QByteArray и QVector<QByteArray> записываются как есть.
	 *   - QString и QVector<QString> записываются в кодировке `latin1`.
	 * - `AT`:
	 *   - DcmTagKey преобразуется в два 16-ти битных значения.
	 *   - quint16 - Записываются в неизменном виде.
	 *   - quint32  - Записываются как пара 16-ти битных значений.
	 *   - QByteArray, QString - Разбираются в формате #CDicomTagInfo::fromUserInput
	 *   - QVector допускается для всех типов перечисленных выше.
	 * - `CS`:
	 *   - QByteArray записываются как есть.
	 *   - QString записываются в кодировке `latin1`.
	 *   - quint8, qint16, quint16, qint32, quint32, qint64, quint64, double, float - Записываются в виде текста.
	 *   - bool - Записывается как "YES" или "NO"
	 *   - QVector допускается для всех типов перечисленных выше.
	 * - `DA`, `DT`, `TM`:
	 *   - QDate, QTime, QDateTime, DicomDate, DicomTime, DicomDateTime - преобразуются в соответствующее представление
	 *     в DICOM. Причем, если поступил тип дата-время для VR `DA` и `TM`, то извлекается только необходимая часть
	 *     "дата" или "время" соответственно.
	 *   - QPair<QDate, QDate>, QPair<QTime, QTime>, QPair<QDateTime, QDateTime>, DicomDateRange, DicomTimeRange,
	 *     DicomDateTimeRange - (только если стоит флаг #Flags::IsQueryRetrieve) преобразуются в соответствующее
	 *     представление в DICOM.
	 *   - QByteArray - Записывается как есть
	 *   - QString записываются в кодировке `latin1`.
	 *   - qint32, quint32, qint64, quint64, double, float - Воспринимается как количество секунд с начала эпохи
	 *   - QVector допускается для всех типов перечисленных выше.
	 * - `DS`, `IS`:
	 *   - quint8, qint16, quint16, qint32, quint32, qint64, quint64, double, float - Преобразуется в текстовый
	 *     формат с проверкой переполнения. Если произошло переполнение, то ошибка.
	 *   - QByteArray записывается как есть.
	 *   - QString записывается в кодировке `latin1`.
	 *   - QVector допускается для всех типов перечисленных выше.
	 * - `FL`, `FD`, `SL`, `SS`, `SV`, `UV`:
	 *   - quint8, qint16, quint16, qint32, quint32, qint64, quint64, double, float - Записываются с приведением типа
	 *     к типу VR. Если приведение не удалось (переполнение), то ошибка.
	 *   - QByteArray, QString - Записываются с преобразованием текста в число. Если преобразование не удалось или
	 *     тип данных VR переполнен, то ошибка.
	 *   - QVector допускается для всех типов перечисленных выше.
	 * - `LO`, `PN`, `SH`, `UC`:
	 *   - QByteArray записывается с переводом кодировки в UTF-8, затем в #encoding.
	 *   - QString записывается в кодировке #encoding.
	 *   - quint8, qint16, quint16, qint32, quint32, qint64, quint64, double, float - Записываются в виде текста
	 *   - QUuid - Записывается в виде текста без фигурных скобок.
	 *   - DcmTag - Записывается в формате `(gggg,eeee)`
	 *   - DcmTagKey - Записывается в формате `(gggg,eeee)`
	 *   - DicomFindIntRange<int> - записывается как два числа с разделяющим символом тире '-'.
	 *   - bool - записывается как "YES" или "NO"
	 *   - QVector допускается для всех типов перечисленных выше.
	 * - `LT`, `ST`, `UT`:
	 *   - QByteArray записывается с переводом кодировки в UTF-8, затем в #encoding.
	 *   - QString записывается в кодировке #encoding.
	 *   - quint8, qint16, quint16, qint32, quint32, qint64, quint64, double, float - Записываются в виде текста
	 *   - QUuid - Записывается в виде текста без фигурных скобок.
	 *   - DcmTag - Записывается в формате `(gggg,eeee)`
	 *   - DcmTagKey - Записывается в формате `(gggg,eeee)`
	 *   - QUrl - записывается как есть.
	 *   - DicomFindIntRange<int> - записывается как два числа с разделяющим символом тире '-'.
	 *   - bool - записывается как "YES" или "NO"
	 * - `OB`, `OD`, `OF`, `OL`, `OV`, `OW`, `UN`:
	 *   - quint8, qint16, quint16, qint32, quint32, qint64, quint64, double, float - Записываются только в случае
	 *     точного совпадения типа исключая различие знаковый/беззнаковый.
	 *   - QByteArray - Записывается как байтовое значение атрибута в порядке следования байт этой машины. Проверяется
	 *     только кратность длинны на соответствие типу данных DICOM.
	 * - `SQ`:
	 *   - QVector<DicomVariantMap>
	 *   - DicomVariantMap
	 *   - QJsonObject, QJsonArray, QJsonValue с типом `object` или `array`
	 * - `UI`:
	 *   - QByteArray записываются как есть.
	 *   - QString записываются в кодировке `latin1`.
	 *   - quint8, qint16, quint16, qint32, quint32, qint64, quint64 - Записываются в виде текста.
	 *   - double, float - Записываются в виде текста всегда в десятичной нотации.
	 *   - QUuid - Преобразуется к OID. Если преобразование не успешно, то ошибка.
	 *   - QVector допускается для всех типов перечисленных выше.
	 * - `UL`:
	 *   - quint8, qint16, quint16, qint32, quint32, qint64, quint64, double, float - Записываются с приведением типа
	 *     к типу VR. Если приведение не удалось (переполнение), то ошибка.
	 *   - QByteArray, QString - Записываются с преобразованием текста в число. Если преобразование не удалось или
	 *     тип данных VR переполнен, то ошибка.
	 *   - DcmTagKey - Записывается в виде одного значения (старшие 16 бит - номер группы, младшие - номер элемента)
	 *   - QVector допускается для всех типов перечисленных выше.
	 * - `UR`:
	 *   - QByteArray записываются как есть.
	 *   - QString записываются в кодировке `latin1`.
	 *   - QUrl приводится к тексту
	 * - `US`:
	 *   - quint8, qint16, quint16, qint32, quint32, qint64, quint64, double, float - Записываются с приведением типа
	 *     к типу VR. Если приведение не удалось (переполнение), то ошибка.
	 *   - QByteArray, QString - Записываются с преобразованием текста в число. Если преобразование не удалось или
	 *     тип данных VR переполнен, то ошибка.
	 *   - bool записывается как "0" или "1"
	 *   - DcmTagKey - Записывается в виде двух значений (номер группы и элемента)
	 *   - QVector допускается для всех типов перечисленных выше.
	 */
	CAPRESULT put(const DcmTag& tag, const QVariant& val, bool bReplace = true);

	/** Создает или обновляет атрибут \a tag с указанным списком значений \a val (DS, FL, OF)
	 *
	 * Поддерживаемые VR:
	 * - Полноценно: DS, FL, OF
	 * - Конвертация при отсутствии #Flags::StrictMode: FD, IS, OD, SL, SS, SV, UL, US, UV
	 *
	 * \param tag Тэг атрибута. Если указан частный атрибут и #isResolvingPrivateTags == \c true,
	 * то идентификатор проходит через #reservePrivateTag.
	 * \param val Записываемое значение.
	 * \param bReplace Заменять ли существующее значение.
	 * \return
	 * - #capElementExists - Атрибут уже существует (если \a bReplace == \c false)
	 * - #capFormatError - Невозможно преобразовать число без потерь или входящий тип данных в VR атрибута.
	 * - #capDicomDimseTypeIncompatibleWithVr - VR несовместим с типом данных (также, если есть флаг #Flags::StrictMode
	 *   и VR не из списка поддерживаемых полноценно)
	 * - Другой код в случае ошибок обработки Private Creator или ошибок в DCMTK
	 */
	CAPRESULT put(const DcmTag& tag, const QVector<float>& val, bool bReplace = true);

	/** Создает или обновляет атрибут \a tag с указанным списком значений \a val (DS, FD, OD)
	 *
	 * Поддерживаемые VR:
	 * - Полноценно: DS, FD, OD
	 * - Конвертация при отсутствии #Flags::StrictMode: FL, IS, OF, SL, SS, SV, UL, US, UV
	 *
	 * \sa put(const DcmTag&, const QVector<float>&, bool)
	 */
	CAPRESULT put(const DcmTag& tag, const QVector<double>& val, bool bReplace = true);

	/** Создает или обновляет атрибут \a tag с указанным списком значений \a val (OB, UN)
	 *
	 * Поддерживаемые VR:
	 * - Полноценно: OB, UN
	 * - Конвертация при отсутствии #Flags::StrictMode: DS, FD, FL, IS, SL, SS, SV, UL, US, UV
	 *
	 * \sa put(const DcmTag&, const QVector<float>&, bool)
	 */
	CAPRESULT put(const DcmTag& tag, const QVector<quint8>& val, bool bReplace = true);

	/** Создает или обновляет атрибут \a tag с указанным списком значений \a val (AT, OW, US)
	 *
	 * Поддерживаемые VR:
	 * - Полноценно: AT, OW, US
	 * - Конвертация при отсутствии #Flags::StrictMode: DS, FD, FL, IS, SL, SS, SV, UL, UV
	 *
	 * \sa put(const DcmTag&, const QVector<float>&, bool)
	 */
	CAPRESULT put(const DcmTag& tag, const QVector<qint16>& val, bool bReplace = true);

	/** Создает или обновляет атрибут \a tag с указанным списком значений \a val (SS)
	 *
	 * Поддерживаемые VR:
	 * - Полноценно: SS
	 * - Конвертация при отсутствии #Flags::StrictMode: DS, FD, FL, IS, SL, SV, UL, US, UV
	 *
	 * \sa put(const DcmTag&, const QVector<float>&, bool)
	 */
	CAPRESULT put(const DcmTag& tag, const QVector<quint16>& val, bool bReplace = true);

	/** Создает или обновляет атрибут \a tag с указанным списком значений \a val (IS, SL)
	 *
	 * Поддерживаемые VR:
	 * - Полноценно: IS, SL
	 * - Конвертация при отсутствии #Flags::StrictMode: DS, FD, FL, SS, SV, US, UL, UV
	 *
	 * \sa put(const DcmTag&, const QVector<float>&, bool)
	 */
	CAPRESULT put(const DcmTag& tag, const QVector<qint32>& val, bool bReplace = true);

	/** Создает или обновляет атрибут \a tag с указанным списком значений \a val (OL, UL)
	 *
	 * Поддерживаемые VR:
	 * - Полноценно: OL, UL, AT (group в старших 16 битах, element в младших)
	 * - Конвертация при отсутствии #Flags::StrictMode: DS, FD, FL, IS, SL, SS, SV, US, UV
	 *
	 * \sa put(const DcmTag&, const QVector<float>&, bool)
	 */
	CAPRESULT put(const DcmTag& tag, const QVector<quint32>& val, bool bReplace = true);

	/** Создает или обновляет атрибут \a tag с указанным списком значений \a val (SV)
	 *
	 * Поддерживаемые VR:
	 * - Полноценно: SV
	 * - Конвертация при отсутствии #Flags::StrictMode: DS, FD, FL, IS, SS, SL, US, UL, UV
	 *
	 * \sa put(const DcmTag&, const QVector<float>&, bool)
	 */
	CAPRESULT put(const DcmTag& tag, const QVector<qint64>& val, bool bReplace = true);

	/** Создает или обновляет атрибут \a tag с указанным списком значений \a val (UV, OV)
	 *
	 * Поддерживаемые VR:
	 * - Полноценно: UV, OV
	 * - Конвертация при отсутствии #Flags::StrictMode: DS, FD, FL, IS, SS, SL, SV, US, UL
	 *
	 * \sa put(const DcmTag&, const QVector<float>&, bool)
	 */
	CAPRESULT put(const DcmTag& tag, const QVector<quint64>& val, bool bReplace = true);

	/** Создает или обновляет атрибут \a tag с указанным списком значений \a val (AT)
	 *
	 * Поддерживается только VR = AT.
	 *
	 * \sa put(const DcmTag&, const QVector<float>&, bool)
	 */
	CAPRESULT put(const DcmTag& tag, const QVector<DcmTagKey>& val, bool bReplace = true);

	/** Создает или обновляет атрибут \a tag с указанным списком значений \a val
	 *
	 * Поддерживаемые VR:
	 * - Текстовые с кодировкой #encoding: LO, LT, PN, SH, ST, UC, UT
	 * - Текстовые с кодировкой "latin1": AE, AS, CS, DA, DS, DT, IS, TM, UI, UR
	 * - Бинарные c преобразованием текст => число: FL, FD, OB, OD, OF, OL, OV, OW, SL, SS, SV, UL, UN, US, UV
	 *
	 * \sa put(const DcmTag&, const QVector<float>&, bool)
	 */
	CAPRESULT put(const DcmTag& tag, const QVector<QString>& val, bool bReplace = true);

	/** Создает или обновляет атрибут \a tag с указанным списком значений \a val
	 *
	 * Поддерживаемые VR:
	 * - Текстовые с перекодировкой из UTF-8 в #encoding: LO, LT, PN, SH, ST, UC, UT
	 * - Текстовые с без перекодировки: AE, AS, CS, DA, DS, DT, IS, TM, UI, UR
	 * - Бинарные c преобразованием текст => число: FL, FD, OB, OD, OF, OL, OV, OW, SL, SS, SV, UL, UN, US, UV
	 *
	 * \sa put(const DcmTag&, const QVector<float>&, bool)
	 */
	CAPRESULT put(const DcmTag& tag, const QVector<QByteArray>& val, bool bReplace = true);

public: // Запись одного значения атрибута

	/** Устанавливает единственное значение \a val сиквенсу \a tag.
	 *
	 * Поддерживаемые VR:
	 * - SQ
	 *
	 * \param tag Идентификатор атрибута. Если указан частный атрибут и #isResolvingPrivateTags == \c true,
	 * то идентификатор проходит через #reservePrivateTag.
	 * \param[in] Устанавливаемый датасет. Специфика:
	 * - Если DCMTK объект датасета в \a val уже находится в устанавливаемом сиквенсе, то все остальные значения
	 *   сиквенса удаляются.
	 * - Если датасет \a val не владеет DCMTK объектом, то производится глубокая копия датасета.
	 * - В датасете \a val происходит преобразование кодировки текста к текущей, если это требуется.
	 * \param bReplace Заменять ли существующее значение.
	 */
	CAPRESULT put(const DcmTag& tag, CDataset&& val, bool bReplace = true);

	/** Создает или перезаписывает атрибут \a tag с указанным значением \a val (DS, FL, OF)
	 *
	 * Поддерживаемые VR:
	 * - Полноценно: DS, FL, OF
	 * - Конвертация при отсутствии #Flags::StrictMode: FD, IS, OD, SL, SS, SV, UL, US, UV
	 *
	 * \param tag Тэг атрибута. Если указан частный атрибут и #isResolvingPrivateTags == \c true,
	 * то идентификатор проходит через #reservePrivateTag.
	 * \param val Записываемое значение
	 * \param bReplace Заменять ли существующее значение.
	 * \return
	 * - #capElementExists - Атрибут уже существует (если \a bReplace == \c false)
	 * - #capFormatError - Невозможно преобразовать число без потерь или входящий тип данных в VR атрибута.
	 * - #capDicomDimseTypeIncompatibleWithVr - VR несовместим с типом данных (также, если есть флаг #Flags::StrictMode
	 *   и VR не из списка поддерживаемых полноценно)
	 * - Другой код в случае ошибок обработки Private Creator или ошибок в DCMTK
	 */
	CAPRESULT put(const DcmTag& tag, float val, bool bReplace = true);

	/** Создает или перезаписывает атрибут \a tag с указанным значением \a val (DS, FD, OD)
	 *
	 * Поддерживаемые VR:
	 * - Полноценно: DS, FD, OD
	 * - Конвертация при отсутствии #Flags::StrictMode: FL, IS, OF, SL, SS, SV, UL, US, UV
	 *
	 * \sa put(const DcmTag&, float, bool)
	 */
	CAPRESULT put(const DcmTag& tag, double val, bool bReplace = true);

	/** Создает или перезаписывает атрибут \a tag с указанным значением \a val (OB, UN)
	 *
	 * Поддерживаемые VR:
	 * - Полноценно: OB, UN
	 * - Конвертация при отсутствии #Flags::StrictMode: DS, FD, FL, IS, SL, SS, SV, UL, US, UV
	 *
	 * \sa put(const DcmTag&, float, bool)
	 */
	CAPRESULT put(const DcmTag& tag, quint8 val, bool bReplace = true);

	/** Создает или перезаписывает атрибут \a tag с указанным значением \a val (AT, OW, US)
	 *
	 * Поддерживаемые VR:
	 * - Полноценно: AT, OW, US
	 * - Конвертация при отсутствии #Flags::StrictMode: DS, FD, FL, IS, SL, SS, SV, UL, UV
	 *
	 * \sa put(const DcmTag&, float, bool)
	 */
	CAPRESULT put(const DcmTag& tag, qint16 val, bool bReplace = true);

	/** Создает или перезаписывает атрибут \a tag с указанным значением \a val (OW, US)
	 *
	 * Поддерживаемые VR:
	 * - Полноценно: OW, US
	 * - Конвертация при отсутствии #Flags::StrictMode: DS, FD, FL, IS, SL, SS, SV, UL, UV
	 *
	 * \sa put(const DcmTag&, float, bool)
	 */
	CAPRESULT put(const DcmTag& tag, quint16 val, bool bReplace = true);

	/** Создает или перезаписывает атрибут \a tag с указанным значением \a val (IS, SL)
	 *
	 * Поддерживаемые VR:
	 * - Полноценно: IS, SL
	 * - Конвертация при отсутствии #Flags::StrictMode: DS, FD, FL, SS, SV, US, UL, UV
	 *
	 * \sa put(const DcmTag&, float, bool)
	 */
	CAPRESULT put(const DcmTag& tag, qint32 val, bool bReplace = true);

	/** Создает или перезаписывает атрибут \a tag с указанным значением \a val (OL, UL, AT)
	 *
	 * Поддерживаемые VR:
	 * - Полноценно: OL, UL, AT (group в старших 16 битах, element в младших)
	 * - Конвертация при отсутствии #Flags::StrictMode: DS, FD, FL, IS, SL, SS, SV, US, UV
	 *
	 * \sa put(const DcmTag&, float, bool)
	 */
	CAPRESULT put(const DcmTag& tag, quint32 val, bool bReplace = true);

	/** Создает или перезаписывает атрибут \a tag с указанным значением \a val (SV)
	 *
	 * Поддерживаемые VR:
	 * - Полноценно: SV
	 * - Конвертация при отсутствии #Flags::StrictMode: DS, FD, FL, IS, SS, SL, US, UL, UV
	 *
	 * \sa put(const DcmTag&, float, bool)
	 */
	CAPRESULT put(const DcmTag& tag, qint64 val, bool bReplace = true);

	/** Создает или перезаписывает атрибут \a tag с указанным значением \a val (UV, OV)
	 *
	 * Поддерживаемые VR:
	 * - Полноценно: UV, OV
	 * - Конвертация при отсутствии #Flags::StrictMode: DS, FD, FL, IS, SS, SL, SV, US, UL
	 *
	 * \sa put(const DcmTag&, float, bool)
	 */
	CAPRESULT put(const DcmTag& tag, quint64 val, bool bReplace = true);

	/** Создает или перезаписывает атрибут \a tag с указанным значением \a val (AT)
	 *
	 * Поддерживается только VR = AT.
	 *
	 * \sa put(const DcmTag&, float, bool)
	 */
	CAPRESULT put(const DcmTag& tag, const DcmTagKey& val, bool bReplace = true);

	/** Создает или перезаписывает атрибут \a tag с указанным значением \a val (DA)
	 *
	 * Поддерживается только VR = DA.
	 *
	 * \sa put(const DcmTag&, float, bool)
	 */
	CAPRESULT put(const DcmTag& tag, const QDate& val, bool bReplace = true);

	/** Создает или перезаписывает атрибут \a tag с указанным значением \a val (TM)
	 *
	 * Поддерживается только VR = TM.
	 *
	 * \sa put(const DcmTag&, float, bool)
	 */
	CAPRESULT put(const DcmTag& tag, const QTime& val, bool bReplace = true);

	/** Создает или перезаписывает атрибут \a tag с указанным значением \a val (DT)
	 *
	 * Поддерживается только VR = DT.
	 *
	 * \sa put(const DcmTag&, float, bool)
	 */
	CAPRESULT put(const DcmTag& tag, const QDateTime& val, bool bReplace = true);

	/** Создает или перезаписывает атрибут \a tag с указанным значением \a val
	 *
	 * Поддерживаемые VR:
	 * - Текстовые с кодировкой #encoding: LO, LT, PN, SH, ST, UC, UT
	 * - Текстовые с кодировкой "latin1": AE, AS, CS, DA, DS, DT, IS, TM, UI, UR
	 * - Бинарные c преобразованием текст => число: FL, FD, OB, OD, OF, OL, OV, OW, SL, SS, SV, UL, UN, US, UV
	 *
	 * Этот метод ожидает только одно значение! Если требуется установить сразу несколько
	 * значений в виде текста, то используйте #putText(const DcmTag&, const QString&, bool).
	 *
	 * \sa put(const DcmTag&, float, bool)
	 */
	CAPRESULT put(const DcmTag& tag, const QString& val, bool bReplace = true);

	/** Создает или перезаписывает атрибут \a tag с указанным значением \a val
	 *
	 * Поддерживаемые VR:
	 * - Текстовые с кодировкой #encoding: LO, LT, PN, SH, ST, UC, UT
	 * - Текстовые с кодировкой "latin1": AE, AS, CS, DA, DS, DT, IS, TM, UI, UR
	 * - Бинарные c преобразованием текст => число: FL, FD, OB, OD, OF, OL, OV, OW, SL, SS, SV, UL, UN, US, UV
	 *
	 * Этот метод ожидает только одно значение! Если требуется установить сразу несколько
	 * значений в виде текста, то используйте #putText(const DcmTag&, QStringView, bool).
	 *
	 * \sa put(const DcmTag&, float, bool)
	 */
	CAPRESULT put(const DcmTag& tag, QStringView val, bool bReplace = true);

	/** Создает или перезаписывает атрибут \a tag с указанным значением \a val
	 *
	 * Поддерживаемые VR:
	 * - Текстовые с перекодировкой из UTF-8 в #encoding: LO, LT, PN, SH, ST, UC, UT
	 * - Текстовые с без перекодировки: AE, AS, CS, DA, DS, DT, IS, TM, UI, UR
	 * - Бинарные c преобразованием текст => число: FL, FD, OB, OD, OF, OL, OV, OW, SL, SS, SV, UL, UN, US, UV
	 *
	 * Этот метод ожидает только одно значение! Если требуется установить сразу несколько
	 * значений в виде текста, то используйте #putText(const DcmTag&, const QByteArray&, bool).
	 *
	 * \sa put(const DcmTag&, float, bool)
	 */
	CAPRESULT put(const DcmTag& tag, const QByteArray& val, bool bReplace = true);

	/** Создает или перезаписывает атрибут \a tag с указанным значением \a val
	 *
	 * Поддерживаемые VR:
	 * - Текстовые с перекодировкой из UTF-8 в #encoding: LO, LT, PN, SH, ST, UC, UT
	 * - Текстовые с без перекодировки: AE, AS, CS, DA, DS, DT, IS, TM, UI, UR
	 * - Бинарные c преобразованием текст => число: FL, FD, OB, OD, OF, OL, OV, OW, SL, SS, SV, UL, UN, US, UV
	 *
	 * Этот метод ожидает только одно значение! Если требуется установить сразу несколько
	 * значений в виде текста, то используйте #putText(const DcmTag&, std::string_view, bool).
	 *
	 * \sa put(const DcmTag&, float, bool)
	 */
	CAPRESULT put(const DcmTag& tag, std::string_view val, bool bReplace = true);

	/** Создает или перезаписывает атрибут \a tag с указанным значением \a val
	 *
	 * Поддерживаемые VR:
	 * - Текстовые с перекодировкой из UTF-8 в #encoding: LO, LT, PN, SH, ST, UC, UT
	 * - Текстовые с без перекодировки: AE, AS, CS, DA, DS, DT, IS, TM, UI, UR
	 * - Бинарные c преобразованием текст => число: FL, FD, OB, OD, OF, OL, OV, OW, SL, SS, SV, UL, UN, US, UV
	 *
	 * Этот метод ожидает только одно значение! Если требуется установить сразу несколько
	 * значений в виде текста, то используйте #putText(const DcmTag&, std::string_view, bool).
	 *
	 * \sa put(const DcmTag&, float, bool)
	 */
	CAPRESULT put(const DcmTag& tag, const char* val, bool bReplace = true);

	/** Создает или перезаписывает атрибут \a tag с указанным значением \a val
	 *
	 * Поддерживаемые VR:
	 * - Текстовые с перекодировкой из UTF-8 в #encoding: LO, LT, PN, SH, ST, UC, UT
	 * - Текстовые с без перекодировки: AE, AS, CS, DA, DS, DT, IS, TM, UI, UR
	 * - Бинарные c преобразованием текст => число: FL, FD, OB, OD, OF, OL, OV, OW, SL, SS, SV, UL, UN, US, UV
	 *
	 * Этот метод ожидает только одно значение! Если требуется установить сразу несколько
	 * значений в виде текста, то используйте #putText(const DcmTag&, std::string_view, bool).
	 *
	 * \sa put(const DcmTag&, float, bool)
	 */
	template <size_t Size>
	inline CAPRESULT put(const DcmTag& tag, const char (&val)[Size], bool bReplace = true);

private:
	class Private;

	friend class CDatasetSequence;

	/** Объект конфигурации датасета.*/
	struct Config
	{
		CDicomEncoding encoding;					 ///< Текущая кодировка текста.
		qint32 tzOffset = LOCAL_TIME_OFFSET;		 ///< Текущий офсет времени от UTC в секундах.
		bool tzOffsetInDataset = false;				 ///< Находится ли значение #tzOffset в атрибуте датасета.
		int flags = DEFAULT_FLAGS;					 ///< Флаги поведения датасета. Битовая маска из #Flags
		QChar pnDelimiter = VALUES_DELIMITER<QChar>; ///< Разделитель нескольких значений для VR = PN
	};

	/** Выполняет преобразование (адаптирование) атрибутов датасета к настройкам другого датасета, в который текущий
	 * вступает в качестве элемента сиквенса.
	 *
	 * Если датасет назначения #root неизвестен(nullptr) или равен #m_root, то метод ничего не выполняет.
	 *
	 * Производимые действия:
	 * - Вызывается #changeEncoding
	 * - Вызывается #changeTimezoneOffsetFromUtc
	 * - Удаляются атрибуты Specific Character Set (0008,0005) и Timezone Offset from UTC (0008,0201)
	 * - Устанавливается новый #m_root и #m_config
	 */
	void changeRoot(CDataset* newRoot);

	Type m_type;		 ///< Тип датасета.
	DcmItem* m_dcm {};	 ///< Указатель на обернутый объект DCMTK (DcmItem, DcmDataset, DcmMetaInfo).
	bool m_dcmOwned {};	 ///< Владеет ли текущий объект указателем #m_dcm
	CDataset* m_root {}; ///< Указатель на корневой датасет. Будет `this`, если этот датасет корневой.
	Config m_config;	 ///< Конфигурация. Используется только у корневого датасета. У остальных - балласт :)
};



//-----------------------------------------------------------------------------
template<class T> inline T CDataset::getBytes(const DcmTag& tag) const
{
	T rv {};
	getBytes(tag, rv);
	return rv;
}

//-----------------------------------------------------------------------------
template<class T> inline T CDataset::getBytesDefaulted(const DcmTag& tag, T&& defaultValue) const
{
	T rv {};
	if (CAP_FAILED(getBytes(tag, rv)))
	{
		using std::swap;
		swap(rv, defaultValue);
	}
	return rv;
}

//-----------------------------------------------------------------------------
template<class T> inline T CDataset::getText(const DcmTag& tag) const
{
	T rv {};
	getText(tag, rv);
	return rv;
}

//-----------------------------------------------------------------------------
template<class T> inline T CDataset::getTextDefaulted(const DcmTag& tag, T&& defaultValue) const
{
	T rv {};
	if (CAP_FAILED(getText(tag, rv)))
	{
		using std::swap;
		swap(rv, defaultValue);
	}
	return rv;
}

//-----------------------------------------------------------------------------
template<class T> inline T CDataset::get(const DcmTagKey& tag) const
{
	T rv {};
	if (CAP_FAILED(get(tag, rv)))
		rv = T();
	return rv;
}

//-----------------------------------------------------------------------------
template<class T> inline T CDataset::getDefaulted(const DcmTagKey& tag, T&& defaultValue) const
{
	T rv {};
	if (CAP_FAILED(get(tag, rv)))
	{
		using std::swap;
		swap(rv, defaultValue);
	}
	return rv;
}

//-----------------------------------------------------------------------------
template<size_t Size>
inline CAPRESULT CDataset::putBytes(const DcmTag &tag, const char (&val)[Size], bool bReplace)
{
	return putBytes(tag, std::string_view(val, Size), bReplace);
}

//-----------------------------------------------------------------------------
template<size_t Size>
inline CAPRESULT CDataset::putText(const DcmTag &tag, const char (&val)[Size], bool bReplace)
{
	return putText(tag, std::string_view(val, Size), bReplace);
}

//-----------------------------------------------------------------------------
template<size_t Size>
CAPRESULT CDataset::put(const DcmTag &tag, const char (&val)[Size], bool bReplace)
{
	return put(tag, std::string_view(val, Size), bReplace);
}

//-----------------------------------------------------------------------------
inline uint qHash(const DcmTag& tag, uint seed = 0)
{
	return qHash(tag.hash(), seed);
}

//-----------------------------------------------------------------------------
inline uint qHash(const DcmTagKey& tag, uint seed = 0)
{
	return qHash(tag.hash(), seed);
}

//-----------------------------------------------------------------------------
QDebug operator<< (QDebug dbg, const DcmTag& tag)
{
	QDebugStateSaver state(dbg);
	dbg.nospace() << "DcmTag(" << tag.getGTag() << ", " << tag.getETag() << ", " << DcmVR(tag.getEVR()).getVRName();
	const char* pc = tag.getPrivateCreator();
	if (pc)
		dbg << ", pc=" << pc;
	dbg << ")";
	return dbg;
}

//-----------------------------------------------------------------------------
QDebug operator<< (QDebug dbg, const DcmTagKey& tag)
{
	QDebugStateSaver state(dbg);
	dbg.nospace() << "DcmTag(" << tag.getGroup() << ", " << tag.getElement() << ")";
	return dbg;
}

CAP_DICOMLIB_EXPORT QDataStream& operator<< (QDataStream&, const DcmTag&);
CAP_DICOMLIB_EXPORT QDataStream& operator>> (QDataStream&, DcmTag&);
CAP_DICOMLIB_EXPORT QDataStream& operator<< (QDataStream&, const DcmTagKey&);
CAP_DICOMLIB_EXPORT QDataStream& operator>> (QDataStream&, DcmTagKey&);

Q_DECLARE_METATYPE(DcmTag);
Q_DECLARE_METATYPE(DcmTagKey);
Q_DECLARE_METATYPE(CDatasetPtr);
Q_DECLARE_METATYPE(DatasetVariantMap);
