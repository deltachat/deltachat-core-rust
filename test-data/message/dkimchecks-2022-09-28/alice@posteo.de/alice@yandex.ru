Authentication-Results: posteo.de; dmarc=pass (p=none dis=none) header.from=yandex.ru
Authentication-Results: posteo.de; spf=pass smtp.mailfrom=yandex.ru
Authentication-Results: posteo.de;
	dkim=pass (1024-bit key) header.d=yandex.ru header.i=@yandex.ru header.b=XsBIC1C8;
	dkim-atps=neutral
Authentication-Results: iva4-143b1447cf50.qloud-c.yandex.net; dkim=pass header.i=@yandex.ru
From: <alice@yandex.ru>
To: <alice@posteo.de>
