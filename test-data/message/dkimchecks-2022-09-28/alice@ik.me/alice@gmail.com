Authentication-Results: mx.infomaniak.com; dmarc=pass (p=none dis=none) header.from=gmail.com
Authentication-Results: mx.infomaniak.com;
	dkim=pass (2048-bit key; unprotected) header.d=gmail.com header.i=@gmail.com header.b="HII5WJV8";
	dkim-atps=neutral
From: <alice@gmail.com>
To: <alice@ik.me>
Authentication-Results: mx.infomaniak.com; spf=pass smtp.mailfrom=gmail.com
