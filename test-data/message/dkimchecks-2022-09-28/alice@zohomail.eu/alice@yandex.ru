Authentication-Results: mx.zohomail.eu;
	dkim=pass;
	spf=pass (zohomail.eu: domain of _spf.yandex.ru designates 37.140.190.195 as permitted sender)  smtp.mailfrom=alice@yandex.ru;
	dmarc=pass(p=none dis=none)  header.from=yandex.ru
ARC-Authentication-Results: i=1; mx.zohomail.eu;
	dkim=pass;
	spf=pass (zohomail.eu: domain of _spf.yandex.ru designates 37.140.190.195 as permitted sender)  smtp.mailfrom=alice@yandex.ru;
	dmarc=pass header.from=<alice@yandex.ru> (p=none dis=none)
Authentication-Results: vla1-b7b6154c4cfd.qloud-c.yandex.net; dkim=pass header.i=@yandex.ru
From: <alice@yandex.ru>
To: <alice@zohomail.eu>
