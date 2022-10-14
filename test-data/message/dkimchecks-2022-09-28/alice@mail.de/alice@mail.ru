Authentication-Results: mxpostfix02.mail.de;
	dkim=pass (2048-bit key; unprotected) header.d=mail.ru header.i=@mail.ru header.b="JKR9v9aG";
	dkim-atps=neutral
From: <alice@mail.ru>
To: bot <alice@mail.de>
Authentication-Results: smtpng1.m.smailru.net; auth=pass smtp.auth=alice@mail.ru smtp.mailfrom=alice@mail.ru
