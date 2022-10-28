Authentication-Results: mx.infomaniak.com; dmarc=pass (p=none dis=none) header.from=yandex.ru
Authentication-Results: mx.infomaniak.com;
	dkim=pass (1024-bit key; unprotected) header.d=yandex.ru header.i=@yandex.ru header.b="lfDaLIBg";
	dkim-atps=neutral
Authentication-Results: iva4-143b1447cf50.qloud-c.yandex.net; dkim=pass header.i=@yandex.ru
From: <alice@yandex.ru>
To: <alice@ik.me>
Authentication-Results: mx.infomaniak.com; spf=pass smtp.mailfrom=yandex.ru
