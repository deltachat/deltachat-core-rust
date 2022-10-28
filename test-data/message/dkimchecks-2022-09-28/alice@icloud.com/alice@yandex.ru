Authentication-Results: bimi.icloud.com; bimi=skipped reason="insufficient dmarc"
Authentication-Results: dmarc.icloud.com; dmarc=pass header.from=yandex.ru
Authentication-Results: dkim-verifier.icloud.com;
	dkim=pass (1024-bit key) header.d=yandex.ru header.i=@yandex.ru header.b=k2jxFMfG
Authentication-Results: spf.icloud.com; spf=pass (spf.icloud.com: domain of alice@yandex.ru designates 5.45.198.239 as permitted sender) smtp.mailfrom=alice@yandex.ru
Authentication-Results: iva4-143b1447cf50.qloud-c.yandex.net; dkim=pass header.i=@yandex.ru
From: <alice@yandex.ru>
To: <alice@icloud.com>
