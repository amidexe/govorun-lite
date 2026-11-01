# Code signing policy

Подпись Windows через SignPath Foundation настраивается: тестовая подпись проверена,
боевой сертификат ещё не выдан. Тестовая сборка не заменяет опубликованные установщики.

Free code signing provided by [SignPath.io](https://about.signpath.io),
certificate by [SignPath Foundation](https://signpath.org).

Подпись предназначена для приложения Говорун и его установщиков, собранных GitHub
Actions из открытого репозитория [amidexe/govorun-lite](https://github.com/amidexe/govorun-lite).
Сторонние библиотеки не подписываются сертификатом проекта.

Автор изменений и проверяющий: [amidexe](https://github.com/amidexe).
Подпись выпуска одобряет [amidexe](https://github.com/amidexe).
Каждый запрос боевой подписи требует ручного одобрения и проверки происхождения сборки.

[Политика обработки данных и условия онлайн-сервисов](https://govorun.club/legal).
