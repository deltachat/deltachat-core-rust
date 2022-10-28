ARC-Authentication-Results: i=1; mx4.messagingengine.com;
    x-csa=none;
    x-me-sender=none;
    x-ptr=pass smtp.helo=forward501o.mail.yandex.net
    policy.ptr=forward501o.mail.yandex.net;
    bimi=skipped (DMARC Policy is not at enforcement);
    arc=none (no signatures found);
    dkim=pass (1024-bit rsa key sha256) header.d=yandex.ru
    header.i=@yandex.ru header.b=mZiIROQD header.a=rsa-sha256
    header.s=mail x-bits=1024;
    dmarc=pass policy.published-domain-policy=none
    policy.applied-disposition=none policy.evaluated-disposition=none
    (p=none,d=none,d.eval=none) policy.policy-from=p
    header.from=yandex.ru;
    iprev=pass smtp.remote-ip=37.140.190.203 (forward501o.mail.yandex.net);
    spf=pass smtp.mailfrom=alice@yandex.ru
    smtp.helo=forward501o.mail.yandex.net
Authentication-Results: mx4.messagingengine.com;
    x-csa=none;
    x-me-sender=none;
    x-ptr=pass smtp.helo=forward501o.mail.yandex.net
      policy.ptr=forward501o.mail.yandex.net
Authentication-Results: mx4.messagingengine.com;
    bimi=skipped (DMARC Policy is not at enforcement)
Authentication-Results: mx4.messagingengine.com;
    arc=none (no signatures found)
Authentication-Results: mx4.messagingengine.com;
    dkim=pass (1024-bit rsa key sha256) header.d=yandex.ru
      header.i=@yandex.ru header.b=mZiIROQD header.a=rsa-sha256
      header.s=mail x-bits=1024;
    dmarc=pass policy.published-domain-policy=none
      policy.applied-disposition=none policy.evaluated-disposition=none
      (p=none,d=none,d.eval=none) policy.policy-from=p
      header.from=yandex.ru;
    iprev=pass smtp.remote-ip=37.140.190.203 (forward501o.mail.yandex.net);
    spf=pass smtp.mailfrom=alice@yandex.ru
      smtp.helo=forward501o.mail.yandex.net
Authentication-Results: iva4-143b1447cf50.qloud-c.yandex.net; dkim=pass header.i=@yandex.ru
From: <alice@yandex.ru>
To: <alice@fastmail.com>
