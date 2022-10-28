Authentication-Results: bimi.icloud.com; bimi=none
Authentication-Results: dmarc.icloud.com; dmarc=pass header.from=mail.ru
Authentication-Results: dkim-verifier.icloud.com;
	dkim=pass (2048-bit key) header.d=mail.ru header.i=@mail.ru header.b=e36cIHLU
Authentication-Results: spf.icloud.com; spf=pass (spf.icloud.com: domain of alice@mail.ru designates 94.100.181.251 as permitted sender) smtp.mailfrom=alice@mail.ru
From: <alice@mail.ru>
To: <alice@icloud.com>
Authentication-Results: smtpng1.m.smailru.net; auth=pass smtp.auth=alice@mail.ru smtp.mailfrom=alice@mail.ru
