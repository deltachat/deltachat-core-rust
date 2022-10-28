Authentication-Results: myt6-95f0aaf173a0.qloud-c.yandex.net; spf=pass (myt6-95f0aaf173a0.qloud-c.yandex.net: domain of hotmail.com designates 40.92.89.36 as permitted sender, rule=[ip4:40.92.0.0/15]) smtp.mail=alice@hotmail.com; dkim=pass header.i=@hotmail.com
ARC-Authentication-Results: i=1; mx.microsoft.com 1; spf=none; dmarc=none;
 dkim=none; arc=none
From: <alice@hotmail.com>
To: <alice@yandex.ru>
