Authentication-Results: mxpostfix03.mail.de;
	dkim=pass (1024-bit key; unprotected) header.d=mailo.com header.i=@mailo.com header.b="WNL8tBaQ";
	dkim-atps=neutral
From: <alice@mailo.com>
To: bot <alice@mail.de>
