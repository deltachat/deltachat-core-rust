Authentication-Results: posteo.de; dmarc=pass (p=reject dis=none) header.from=mail.ru
Authentication-Results: posteo.de; spf=pass smtp.mailfrom=mail.ru
Authentication-Results: posteo.de;
	dkim=pass (2048-bit key) header.d=mail.ru header.i=@mail.ru header.b=SGc3jC2I;
	dkim-atps=neutral
From: <alice@mail.ru>
To: <alice@posteo.de>
Authentication-Results: smtpng1.m.smailru.net; auth=pass smtp.auth=alice@mail.ru smtp.mailfrom=alice@mail.ru
