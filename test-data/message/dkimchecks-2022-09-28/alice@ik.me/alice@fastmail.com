Authentication-Results: mx.infomaniak.com; dmarc=pass (p=none dis=none) header.from=fastmail.com
Authentication-Results: mx.infomaniak.com;
	dkim=pass (2048-bit key; unprotected) header.d=fastmail.com header.i=@fastmail.com header.b="KMpU4FxP";
	dkim=pass (2048-bit key; unprotected) header.d=messagingengine.com header.i=@messagingengine.com header.b="AQlzEcHV";
	dkim-atps=neutral
From: <alice@fastmail.com>
To: <alice@ik.me>
Authentication-Results: mx.infomaniak.com; spf=pass smtp.mailfrom=fastmail.com
