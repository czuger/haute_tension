require 'test_helper'

class PageParserTest < ActiveSupport::TestCase

  def setup
    # @section = create( :downloaded_section )
  end

  # def test_exception_3
  #   @book = create( :downloaded_book, id: 2, url: 'dummy', name: 'dummy' )
  #   @section = create( :downloaded_section, id: 951, downloaded_book_id: @book.id, url: 'http://www.lesitedontvousetesleheros.fr/2014/12/209.html' )
  #   GameCore::SectionParser.new.parse_page( @section )
  #
  #   test_section_content =
  #     [[{:type=>:text, :text=>"Vous roulez sur le sol, au bord de l'inconscience. Des pas précipités retentissent. Vous distinguez vaguement d'horribles visages grimaçants penchés sur vous. De petites mains crasseuses vous giflent. Ce contact répugnant vous réveille. Vous vous mettez sur pied en chancelant et constatez que deux Gnomes parmi les plus laids qu'il vous ait été donné de contempler s'apprêtent à vous attaquer. Leurs visages sont déformés par d'énormes pustules purulentes. Derrière eux, se tient un Bossu d'aspect repoussant. Un côté de son visage n'est plus qu'une grotesque parodie d'humanité : un œil trois fois plus gros que 'la normale remonte jusqu'à l'oreille; la bouche, déformée par les tumeurs, se tord en un rictus morbide ; la pommette est si aplatie qu'on la croirait faite de gélatine. Le Bossu reste à l'écart du combat. Vous affrontez les deux Gnomes séparément, car. la passerelle où vous vous trouvez est trop étroite pour laisser passer plus d'un combattant."}], [{:type=>:text, :form=>:strong, :text=>"PREMIER GNOME EMBAUMEUR"}], [{:type=>:text, :text=>"FORCE: 6 VIE: "}, {:type=>:text, :text=>"10"}], [{:type=>:text, :form=>:strong, :text=>"SECOND GNOME EMBAUMEUR"}], [{:type=>:text, :text=>"FORCE: 9 VIE: "}, {:type=>:text, :text=>"10"}], [{:type=>:a, :href=>"http://lesitedontvousetesleheros.overblog.com/2014/12/179.html", :text=>"Si vous triomphez. "}]]
  #
  #   assert_equal test_section_content, @section.parsed_section.content
  # end
  #
  #
  # def test_rubbish_1
  #   @book = create( :downloaded_book, id: 2, url: 'dummy', name: 'dummy' )
  #   @section = create( :downloaded_section, id: 930, downloaded_book_id: @book.id, url: 'http://www.lesitedontvousetesleheros.fr/2014/12/182.html' )
  #   GameCore::SectionParser.new.parse_page( @section )
  #
  #   test_section_content =
  #     [[{:type=>:text, :text=>"Sans perdre votre sang-froid, vous tirez votre épée. «Dieu me garde! vous écriez-vous. Par le Christ et la Croix ! » Vous assenez un furieux revers à cette abomination. Attention ! Ce monstre se régénère automatiquement: 1 point de Vie par tour !"}], [{:type=>:text, :form=>:strong, :text=>"CORPS-SANS- TÊTE"}], [{:type=>:text, :text=>"FORCE: 8 VIE: 13"}], [{:type=>:a, :href=>"http://lesitedontvousetesleheros.overblog.com/2014/12/114.html", :text=>"Si vous détruisez cet adversaire."}]]
  #
  #   assert_equal test_section_content, @section.parsed_section.content
  # end
  #
  # def test_exception_1
  #   @book = create( :downloaded_book, id: 2, url: 'dummy', name: 'dummy' )
  #   @section = create( :downloaded_section, id: 695, downloaded_book_id: @book.id, url: 'http://lesitedontvousetesleheros.overblog.com/2014/12/520.html' )
  #   GameCore::SectionParser.new.parse_page( @section )
  #
  #   test_section_content =
  #     [[{:type=>:text, :text=>"Au moment où vous allez franchir la porte, des grincements suspects parviennent à vos oreilles. Vous tournez lentement la tête, et évitez de justesse un gros Rat qui bondit sur vous en essayant de vous mordre le visage. L'animal est presque aussi gros qu'un chat ! Trois de ses congénères surgissent des crevasses du sol. Ils ont apparemment épuisé leurs provisions de grain et sont prêts à tout pour se nourrir."}], [{:type=>:a, :href=>"http://lesitedontvousetesleheros.overblog.com/2014/12/391.html", :text=>"Si vous avez pris le flacon sur l'étagère."}], [{:type=>:text, :text=>"Sinon, vous devrez combattre de votre mieux en essayant de ne pas faire de bruit. Pour chaque assaut, lancez deux dés pour vous, et deux fois deux dés pour les Rats (ceci suppose que, sur les quatre, deux sont en mesure de vous toucher à chaque assaut). Vous considérerez donc que vous affrontez deux adversaires par assaut, mais vous ne pourrez en frapper qu'un seul."}], [{:type=>:text, :form=>:strong, :text=>"CHAQUE RAT"}, {:type=>:text, :text=>" CARNIVORE FORCE: 5"}], [{:type=>:a, :href=>"http://lesitedontvousetesleheros.overblog.com/2014/12/224.html", :text=>"Dès que vous aurez remporté deux assauts."}]]
  #
  #   assert_equal test_section_content, @section.parsed_section.content
  # end
  #
  # def test_exception_2
  #   @book = create( :downloaded_book, id: 2, url: 'dummy', name: 'dummy' )
  #   @section = create( :downloaded_section, id: 910, downloaded_book_id: @book.id, url: 'http://lesitedontvousetesleheros.overblog.com/2014/12/536.html' )
  #   GameCore::SectionParser.new.parse_page( @section )
  #
  #   test_section_content =
  #     [[{:type=>:text, :text=>"Vous dégainez votre épée avec l'intention nette de mettre fin aux jours de ce hideux monstre."}], [{:type=>:text, :form=>:strong, :text=>"TÊTE-SANS-CORPS FORCE"}, {:type=>:text, :text=>": 10 VIE: 8"}], [{:type=>:a, :href=>"http://lesitedontvousetesleheros.overblog.com/2014/12/245.html", :text=>"Si vous triomphez."}]]
  #
  #   assert_equal test_section_content, @section.parsed_section.content
  # end
  #
  # def test_remove_rubish_1
  #   @book = create( :downloaded_book, id: 2, url: 'dummy', name: 'dummy' )
  #   @section = create( :downloaded_section, id: 675, downloaded_book_id: @book.id )
  #   GameCore::SectionParser.new.parse_page( @section )
  #
  #   test_section_content =
  #     [[{:type=>:text, :text=>"Un vacarme soutenu vous parvient de la droite de la place. Un groupe important de badauds s'est assemblé au pied d'une estrade surplombée d'un dais de tissu moiré de couleur turquoise. Vous vous glissez jusque-là, traversant non sans mal la foule compacte. Une fois au pied de l'estrade, vous découvrez qu'il s'agit, comme vous le pensiez, d'une vente aux enchères. A votre grande horreur, ce sont des esclaves des deux sexes qui sont l'objet de la vente. Quand vous arrivez, une frêle et jeune Égyptienne aux yeux sombres va être vendue au plus offrant. Le marchand chargé de mener à bien la vente est un grand Nubien à la peau d'ébène luisante de sueur. Il harcèle la malheureuse qui ne se comporte pas comme il le souhaiterait. Au bout d'un moment, il pousse la pauvre fille à terre d'une violente bourrade. « Combien m'offrez-vous pour cette fille de prince, messeigneurs? Elle est digne de vous servir, malgré son air hautain! Oh ça oui, avec quelques bons coups de fouet, elle se pliera à vos moindres désirs l » Quelques hommes dans l'assistance éclatent d'un rire gras, pensant aux sous-entendus de l'immonde Nubien. Au sol, la malheureuse esclave reste digne et fière malgré ses joues mouillées de larmes. Ce spectacle vous est difficilement supportable."}], [{:type=>:text, :text=>"Qu'allez-vous faire ?"}], [{:type=>:a, :href=>"http://lesitedontvousetesleheros.overblog.com/2014/12/39.html", :text=>"Tenter d'acheter la belle esclave pour la libérer de son bourreau ?"}], [{:type=>:a, :href=>"http://lesitedontvousetesleheros.overblog.com/2014/12/134.html", :text=>"Attaquer immédiatement le marchand d'esclaves ?"}], [{:type=>:a, :href=>"http://lesitedontvousetesleheros.overblog.com/2014/12/547.html", :text=>"Vous détourner de ce terrible spectacle et passer votre chemin ?"}]]
  #
  #   assert_equal test_section_content, @section.parsed_section.content
  # end
  #
  # def test_multiline_monster_2
  #   @book = create( :downloaded_book, id: 2, url: 'dummy', name: 'dummy' )
  #   @section = create( :downloaded_section, id: 807, downloaded_book_id: @book.id )
  #   GameCore::SectionParser.new.parse_page( @section )
  #
  #   test_section_content =
  #     [[{:type=>:text, :text=>"Furieux, vous vous précipitez sur les momies en faisant des moulinets avec votre arme redoutable."}], [{:type=>:text, :form=>:strong, :text=>"MOMIE DU PRÊTRE RAH"}], [{:type=>:text, :text=>"FORCE: 7"}], [{:type=>:text, :text=>"VIE: 10"}], [{:type=>:text, :form=>:strong, :text=>"MOMIE DU PRÊTRE SPAH"}], [{:type=>:text, :text=>"FORCE: 6"}], [{:type=>:text, :text=>"VIE: 10"}], [{:type=>:text, :form=>:strong, :text=>"MOMIE DU PRÊTRE DRAH"}], [{:type=>:text, :text=>"FORCE: 6"}], [{:type=>:text, :text=>"VIE: 11"}], [{:type=>:a, :href=>"http://lesitedontvousetesleheros.overblog.com/2014/12/505.html", :text=>"Si vous sortez victorieux de ce combat."}]]
  #
  #   assert_equal test_section_content, @section.parsed_section.content
  # end
  #
  #
  # def test_multiline_monster_1
  #   @book = create( :downloaded_book, id: 2, url: 'dummy', name: 'dummy' )
  #   @section = create( :downloaded_section, id: 844, downloaded_book_id: @book.id, url: 'http://lesitedontvousetesleheros.overblog.com/2014/11/2.html' )
  #   GameCore::SectionParser.new.parse_page( @section )
  #
  #   test_section_content =
  #     [[{:type=>:text, :text=>"«Comment oses-tu me défier, guerrier de pacotille, moi qui ai accompli mille exploits que tu n'as jamais osé imaginer dans tes rêves de gloire les plus fous ! Je n'ai que faire de vos complots religieux ! Ma quête. est beaucoup plus importante à mes yeux que tous vos prêtres pourchassés ou les trésors de vos temples pillés. » Vous dégainez votre épée en prononçant ces paroles et vous vous précipitez sur votre adversaire."}], [{:type=>:text, :form=>:strong, :text=>"NOBLE ÉGYPTIEN"}], [{:type=>:text, :text=>"FORCE: 12"}], [{:type=>:text, :text=>"VIE : 13"}], [{:type=>:a, :href=>"http://lesitedontvousetesleheros.overblog.com/2014/12/231.html", :text=>"Si vous réussissez à vous en défaire."}]]
  #
  #   assert_equal test_section_content, @section.parsed_section.content
  # end
  #
  # def test_link_inclusion
  #   @section = create( :downloaded_section, id: 595 )
  #   GameCore::SectionParser.new.parse_page( @section )
  #
  #   test_section_content =
  #     [
  #       [ {:type=>:text, :text=>
  #         "Le garde refuse de vous répondre et aucune de vos menaces ne lui délie la langue. Bien décidé à trouver une sortie discrète pour quitter le village, vous bâillonnez votre prisonnier "},
  #         {:type=>:a, :href=>"http://lesitedontvousetesleheros.overblog.com/612-8", :text=>"et vous l'abandonnez dans un recoin sombre."} ]
  #     ]
  #   assert_equal test_section_content, @section.parsed_section.content
  # end
  #
  # def test_adjustement
  #   @section = create( :downloaded_section, id: 237 )
  #   GameCore::SectionParser.new.parse_page( @section )
  #
  #   test_section_content =
  #     [
  #       [ { :type=>:text,
  #           :text=> "Vous plongez, le poignard à la main, prêt à affronter le monstre. Le choc est rude : la pieuvre était aussi préparée que vous ! Mais vous connaissez son point faible : son oeil unique. Vous le transpercerez, tuant ainsi la bête sur le coup, si, au cours du combat, vous réussissez un double. A cause de l'eau qui vous entoure, vous mènerez ce combat en retirant 2 points de votre total de Force du moment. Pour chaque 6 que vous réussirez, ôtez 2 points de Force à la pieuvre : vous aurez tranché l'un de ses tentacules. Cependant si vous ne remportez pas ce duel en quatre assauts au plus, vos poumons cèdent sous le manque d'oxygène et vous mourez noyé. Le poulpe vous dévorera."}],
  #       [ {:type=>:text, :form=>:strong, :text=>"PIEUVRE VENIMEUSE"},
  #         {:type=>:text, :text=>"FORCE : 16 VIE : 6 AJUSTEMENT DEGATS : + 1"} ],
  #       [ {:type=>:a, :href=>"http://lesitedontvousetesleheros.overblog.com/243-5", :text=>"Si vous triomphez."} ]
  #     ]
  #                  # :monsters=>
  #                  #   [{:force=>"16", :vie=>"6", :adjustment=>1, :name=>"PIEUVRE VENIMEUSE "}]}
  #
  #   assert_equal test_section_content, @section.parsed_section.content
  # end

end